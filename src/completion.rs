use crate::model::DatabaseMetadata;

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Table,
    Column,
}

#[derive(Debug, Clone)]
pub enum CompletionContext {
    Table,
    TableColumn { table: String },
    Mixed,
}

const KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "JOIN",
    "LEFT JOIN",
    "INNER JOIN",
    "GROUP BY",
    "ORDER BY",
    "LIMIT",
    "INSERT INTO",
    "UPDATE",
    "DELETE FROM",
    "CREATE TABLE",
    "ALTER TABLE",
    "DROP TABLE",
];

const FUNCTIONS: &[&str] = &["COUNT", "SUM", "AVG", "MIN", "MAX", "NOW"];

const MAX_COMPLETIONS: usize = 16;

pub fn completions(
    prefix: &str,
    context: CompletionContext,
    metadata: Option<&DatabaseMetadata>,
) -> Vec<CompletionItem> {
    let prefix = prefix.to_ascii_uppercase();
    let mut items = Vec::new();

    match &context {
        CompletionContext::Table => push_tables(&mut items, &prefix, metadata),
        CompletionContext::TableColumn { table } => {
            push_table_columns(&mut items, &prefix, metadata, table)
        }
        CompletionContext::Mixed => {
            push_tables(&mut items, &prefix, metadata);
            push_columns(&mut items, &prefix, metadata);
        }
    }

    if !matches!(context, CompletionContext::TableColumn { .. }) {
        push_keywords(&mut items, &prefix);
        push_functions(&mut items, &prefix);
    }

    items.truncate(MAX_COMPLETIONS);
    items
}

pub fn normalize_sql_keyword(token: &str) -> Option<&'static str> {
    let token = token.to_ascii_uppercase();
    let keyword = match token.as_str() {
        "SELECT" => "SELECT",
        "FROM" => "FROM",
        "WHERE" => "WHERE",
        "JOIN" => "JOIN",
        "LEFT" => "LEFT",
        "INNER" => "INNER",
        "GROUP" => "GROUP",
        "BY" => "BY",
        "ORDER" => "ORDER",
        "LIMIT" => "LIMIT",
        "INSERT" => "INSERT",
        "INTO" => "INTO",
        "UPDATE" => "UPDATE",
        "DELETE" => "DELETE",
        "CREATE" => "CREATE",
        "ALTER" => "ALTER",
        "DROP" => "DROP",
        "TABLE" => "TABLE",
        "COUNT" => "COUNT",
        "SUM" => "SUM",
        "AVG" => "AVG",
        "MIN" => "MIN",
        "MAX" => "MAX",
        "NOW" => "NOW",
        _ => return None,
    };
    Some(keyword)
}

pub fn completion_context(before_cursor: &str) -> (String, CompletionContext) {
    if let Some((table, prefix)) = dotted_prefix(before_cursor) {
        return (prefix, CompletionContext::TableColumn { table });
    }

    let prefix = current_prefix(before_cursor);
    let previous = previous_word(before_cursor, &prefix).to_ascii_uppercase();
    let context = if matches!(previous.as_str(), "FROM" | "JOIN") {
        CompletionContext::Table
    } else {
        CompletionContext::Mixed
    };
    (prefix, context)
}

pub fn command_hints(prefix: &str) -> Vec<&'static str> {
    const COMMANDS: &[&str] = &["w", "q", "wq", "x", "run", "q!"];
    let prefix = prefix.to_ascii_lowercase();
    COMMANDS
        .iter()
        .copied()
        .filter(|command| prefix.is_empty() || command.starts_with(&prefix))
        .collect()
}

fn push_keywords(items: &mut Vec<CompletionItem>, prefix: &str) {
    items.extend(
        KEYWORDS
            .iter()
            .filter(|value| matches_prefix(value, prefix))
            .map(|value| CompletionItem {
                label: (*value).to_string(),
                insert_text: (*value).to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
            }),
    );
}

fn push_functions(items: &mut Vec<CompletionItem>, prefix: &str) {
    items.extend(
        FUNCTIONS
            .iter()
            .filter(|value| matches_prefix(value, prefix))
            .map(|value| CompletionItem {
                label: (*value).to_string(),
                insert_text: (*value).to_string(),
                kind: CompletionKind::Function,
                detail: None,
            }),
    );
}

fn push_tables(items: &mut Vec<CompletionItem>, prefix: &str, metadata: Option<&DatabaseMetadata>) {
    let Some(metadata) = metadata else {
        return;
    };
    items.extend(
        metadata
            .tables
            .iter()
            .filter(|table| matches_prefix(&table.name, prefix))
            .map(|table| CompletionItem {
                label: table.name.clone(),
                insert_text: table.name.clone(),
                kind: CompletionKind::Table,
                detail: Some(format!("{} cols", table.columns.len())),
            }),
    );
}

fn push_columns(
    items: &mut Vec<CompletionItem>,
    prefix: &str,
    metadata: Option<&DatabaseMetadata>,
) {
    let Some(metadata) = metadata else {
        return;
    };
    for table in &metadata.tables {
        items.extend(
            table
                .columns
                .iter()
                .filter(|column| matches_prefix(&column.name, prefix))
                .map(|column| CompletionItem {
                    label: column.name.clone(),
                    insert_text: column.name.clone(),
                    kind: CompletionKind::Column,
                    detail: Some(format!(
                        "{}.{}:{}",
                        table.name, column.name, column.data_type
                    )),
                }),
        );
    }
}

fn push_table_columns(
    items: &mut Vec<CompletionItem>,
    prefix: &str,
    metadata: Option<&DatabaseMetadata>,
    table_name: &str,
) {
    let Some(metadata) = metadata else {
        return;
    };
    let Some(table) = metadata
        .tables
        .iter()
        .find(|table| table.name.eq_ignore_ascii_case(table_name))
    else {
        return;
    };
    items.extend(
        table
            .columns
            .iter()
            .filter(|column| matches_prefix(&column.name, prefix))
            .map(|column| CompletionItem {
                label: column.name.clone(),
                insert_text: column.name.clone(),
                kind: CompletionKind::Column,
                detail: Some(column.data_type.clone()),
            }),
    );
}

fn matches_prefix(value: &str, prefix: &str) -> bool {
    prefix.is_empty() || value.to_ascii_uppercase().starts_with(prefix)
}

fn current_prefix(before_cursor: &str) -> String {
    before_cursor
        .chars()
        .rev()
        .take_while(|value| value.is_ascii_alphanumeric() || *value == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn previous_word(before_cursor: &str, current_prefix: &str) -> String {
    let prefix_len = current_prefix.chars().count();
    let chars = before_cursor.chars().collect::<Vec<_>>();
    let mut index = chars.len().saturating_sub(prefix_len);
    while index > 0 && !chars[index - 1].is_ascii_alphanumeric() && chars[index - 1] != '_' {
        index -= 1;
    }
    let end = index;
    while index > 0 && (chars[index - 1].is_ascii_alphanumeric() || chars[index - 1] == '_') {
        index -= 1;
    }
    chars[index..end].iter().collect()
}

fn dotted_prefix(before_cursor: &str) -> Option<(String, String)> {
    let chars = before_cursor.chars().collect::<Vec<_>>();
    let mut prefix_start = chars.len();
    while prefix_start > 0
        && (chars[prefix_start - 1].is_ascii_alphanumeric() || chars[prefix_start - 1] == '_')
    {
        prefix_start -= 1;
    }
    if prefix_start == 0 || chars[prefix_start - 1] != '.' {
        return None;
    }
    let mut table_start = prefix_start - 1;
    while table_start > 0
        && (chars[table_start - 1].is_ascii_alphanumeric() || chars[table_start - 1] == '_')
    {
        table_start -= 1;
    }
    let table = chars[table_start..prefix_start - 1]
        .iter()
        .collect::<String>();
    if table.is_empty() {
        return None;
    }
    let prefix = chars[prefix_start..].iter().collect::<String>();
    Some((table, prefix))
}

#[cfg(test)]
mod tests {
    use super::{CompletionContext, completion_context, completions};
    use crate::model::{ColumnMetadata, DatabaseMetadata, TableMetadata};

    fn metadata() -> DatabaseMetadata {
        DatabaseMetadata {
            tables: vec![TableMetadata {
                name: "actor".to_string(),
                columns: vec![
                    ColumnMetadata {
                        name: "actor_id".to_string(),
                        data_type: "smallint".to_string(),
                    },
                    ColumnMetadata {
                        name: "first_name".to_string(),
                        data_type: "varchar".to_string(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn suggests_tables_after_from() {
        let (prefix, context) = completion_context("SELECT * FROM ac");
        let items = completions(&prefix, context, Some(&metadata()));

        assert!(items.iter().any(|item| item.label == "actor"));
    }

    #[test]
    fn suggests_columns_after_table_dot() {
        let (prefix, context) = completion_context("SELECT actor.fi");
        assert!(matches!(context, CompletionContext::TableColumn { .. }));
        let items = completions(&prefix, context, Some(&metadata()));

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "first_name");
        assert_eq!(items[0].detail.as_deref(), Some("varchar"));
    }
}
