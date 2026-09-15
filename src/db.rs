use std::collections::BTreeMap;
use std::time::Instant;

use mysql::prelude::Queryable;
use mysql::{Opts, Pool, PooledConn, Row, Value};

use crate::model::{
    ColumnMetadata, ConnectionProfile, DatabaseMetadata, ExecutionResult, ResultKind, TableMetadata,
};

const MAX_RESULT_ROWS: usize = 200;

pub fn execute_sql(profile: &ConnectionProfile, sql: &str) -> ExecutionResult {
    let started_at = Instant::now();

    match execute_sql_inner(profile, sql) {
        Ok(result) => result,
        Err(error) => ExecutionResult::error(error.to_string(), started_at.elapsed().as_millis()),
    }
}

pub fn load_metadata(profile: &ConnectionProfile) -> anyhow::Result<DatabaseMetadata> {
    let url = build_mysql_url(profile);
    let opts = Opts::from_url(&url)?;
    let pool = Pool::new(opts)?;
    let mut conn = pool.get_conn()?;

    let table_names = conn.exec_map(
        "select table_name from information_schema.tables where table_schema = ? and table_type = 'BASE TABLE' order by table_name",
        (profile.database.clone(),),
        |table_name: String| table_name,
    )?;

    let column_rows = conn.exec_map(
        "select table_name, column_name, data_type from information_schema.columns where table_schema = ? order by table_name, ordinal_position",
        (profile.database.clone(),),
        |(table_name, column_name, data_type): (String, String, String)| {
            (table_name, ColumnMetadata { name: column_name, data_type })
        },
    )?;

    let mut columns_by_table = BTreeMap::<String, Vec<ColumnMetadata>>::new();
    for (table_name, column) in column_rows {
        columns_by_table.entry(table_name).or_default().push(column);
    }

    let tables = table_names
        .into_iter()
        .map(|name| TableMetadata {
            columns: columns_by_table.remove(&name).unwrap_or_default(),
            name,
        })
        .collect();

    Ok(DatabaseMetadata { tables })
}

fn execute_sql_inner(profile: &ConnectionProfile, sql: &str) -> anyhow::Result<ExecutionResult> {
    let url = build_mysql_url(profile);
    let opts = Opts::from_url(&url)?;
    let pool = Pool::new(opts)?;
    let mut conn = pool.get_conn()?;
    let started_at = Instant::now();

    if is_query_sql(sql) {
        execute_query(&mut conn, sql, started_at)
    } else {
        execute_command(&mut conn, sql, started_at)
    }
}

fn execute_query(
    conn: &mut PooledConn,
    sql: &str,
    started_at: Instant,
) -> anyhow::Result<ExecutionResult> {
    let mut result = conn.query_iter(sql)?;
    let columns = result
        .columns()
        .as_ref()
        .iter()
        .map(|column| column.name_str().to_string())
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    let mut truncated = false;

    for row in result.by_ref() {
        let row = row?;
        rows.push(row_to_strings(row));
        if rows.len() >= MAX_RESULT_ROWS {
            truncated = true;
            break;
        }
    }

    Ok(ExecutionResult {
        kind: ResultKind::Query,
        elapsed_ms: started_at.elapsed().as_millis(),
        columns,
        rows,
        affected_rows: None,
        error_message: None,
        truncated,
    })
}

fn execute_command(
    conn: &mut PooledConn,
    sql: &str,
    started_at: Instant,
) -> anyhow::Result<ExecutionResult> {
    conn.query_drop(sql)?;

    Ok(ExecutionResult {
        kind: ResultKind::Command,
        elapsed_ms: started_at.elapsed().as_millis(),
        columns: Vec::new(),
        rows: Vec::new(),
        affected_rows: Some(conn.affected_rows()),
        error_message: None,
        truncated: false,
    })
}

pub fn build_mysql_url(profile: &ConnectionProfile) -> String {
    format!(
        "mysql://{}:{}@{}:{}/{}",
        profile.username, profile.password, profile.host, profile.port, profile.database
    )
}

fn row_to_strings(row: Row) -> Vec<String> {
    row.unwrap().into_iter().map(value_to_string).collect()
}

fn value_to_string(value: Value) -> String {
    match value {
        Value::NULL => "NULL".to_string(),
        Value::Bytes(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Value::Int(value) => value.to_string(),
        Value::UInt(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Double(value) => value.to_string(),
        Value::Date(year, month, day, hour, minute, second, micros) => format!(
            "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{:06}",
            micros
        ),
        Value::Time(is_negative, days, hours, minutes, seconds, micros) => {
            let sign = if is_negative { "-" } else { "" };
            format!(
                "{sign}{days}d {hours:02}:{minutes:02}:{seconds:02}.{:06}",
                micros
            )
        }
    }
}

pub fn is_query_sql(sql: &str) -> bool {
    let normalized = sql.trim().to_ascii_lowercase();
    let keyword = normalized.split_whitespace().next().unwrap_or_default();
    matches!(
        keyword,
        "select" | "show" | "describe" | "desc" | "explain" | "with"
    )
}

#[cfg(test)]
mod tests {
    use super::{execute_sql, load_metadata};
    use crate::model::ConnectionProfile;

    fn sakila_profile() -> ConnectionProfile {
        ConnectionProfile {
            name: "sakila_local".to_string(),
            kind: "mysql".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3306,
            username: "admin".to_string(),
            password: "zsw_123".to_string(),
            database: "sakila".to_string(),
        }
    }

    #[test]
    fn executes_select_against_sakila() {
        let result = execute_sql(&sakila_profile(), "select count(*) as total from actor");

        assert!(
            result.error_message.is_none(),
            "unexpected error: {:?}",
            result.error_message
        );
        assert_eq!(result.columns, vec!["total"]);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], "200");
    }

    #[test]
    fn loads_sakila_metadata() {
        let metadata = load_metadata(&sakila_profile()).expect("metadata should load");
        let actor = metadata
            .tables
            .iter()
            .find(|table| table.name == "actor")
            .expect("actor table should exist");

        assert!(actor.columns.iter().any(|column| column.name == "actor_id"));
        assert!(
            actor
                .columns
                .iter()
                .any(|column| column.name == "first_name")
        );
        assert!(
            actor
                .columns
                .iter()
                .any(|column| column.name == "last_name")
        );
    }
}
