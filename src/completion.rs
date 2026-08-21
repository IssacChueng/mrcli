#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: &'static str,
    pub insert_text: &'static str,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
}

const SQL_COMPLETIONS: &[CompletionItem] = &[
    CompletionItem {
        label: "SELECT",
        insert_text: "SELECT",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "FROM",
        insert_text: "FROM",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "WHERE",
        insert_text: "WHERE",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "JOIN",
        insert_text: "JOIN",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "LEFT JOIN",
        insert_text: "LEFT JOIN",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "INNER JOIN",
        insert_text: "INNER JOIN",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "GROUP BY",
        insert_text: "GROUP BY",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "ORDER BY",
        insert_text: "ORDER BY",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "LIMIT",
        insert_text: "LIMIT",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "INSERT INTO",
        insert_text: "INSERT INTO",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "UPDATE",
        insert_text: "UPDATE",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "DELETE FROM",
        insert_text: "DELETE FROM",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "CREATE TABLE",
        insert_text: "CREATE TABLE",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "ALTER TABLE",
        insert_text: "ALTER TABLE",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "DROP TABLE",
        insert_text: "DROP TABLE",
        kind: CompletionKind::Keyword,
    },
    CompletionItem {
        label: "COUNT",
        insert_text: "COUNT",
        kind: CompletionKind::Function,
    },
    CompletionItem {
        label: "SUM",
        insert_text: "SUM",
        kind: CompletionKind::Function,
    },
    CompletionItem {
        label: "AVG",
        insert_text: "AVG",
        kind: CompletionKind::Function,
    },
    CompletionItem {
        label: "MIN",
        insert_text: "MIN",
        kind: CompletionKind::Function,
    },
    CompletionItem {
        label: "MAX",
        insert_text: "MAX",
        kind: CompletionKind::Function,
    },
    CompletionItem {
        label: "NOW",
        insert_text: "NOW",
        kind: CompletionKind::Function,
    },
];

pub fn sql_completions(prefix: &str) -> Vec<CompletionItem> {
    let prefix = prefix.to_ascii_uppercase();
    SQL_COMPLETIONS
        .iter()
        .filter(|item| prefix.is_empty() || item.label.starts_with(&prefix))
        .take(8)
        .cloned()
        .collect()
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
