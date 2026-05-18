//! SQL 입력 분류기.
//!
//! 사용자가 입력한 T-SQL 텍스트를 보고 read-only / write / DDL 로 단순 분류한다.
//! 프론트엔드 확인 다이얼로그용. 보수적으로 분류 — 알 수 없으면 Write 로 취급.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SqlKind {
    ReadOnly,
    Write,
    Ddl,
    Empty,
    Unknown,
}

/// 결과 — 분류와 함께 어떤 statement 들이 감지되었는지 알림.
#[derive(Debug, Serialize)]
pub struct SqlClassification {
    pub kind: SqlKind,
    /// 감지된 위험 키워드들 (예: INSERT, DROP). 확인 다이얼로그 본문에 표시.
    pub keywords: Vec<String>,
}

static LINE_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)--[^\n]*").unwrap());
static BLOCK_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)/\*.*?\*/").unwrap());
static STR_LITERAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"'([^']|'')*'").unwrap());
static IDENT_QUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?s)\[[^\]]*\]|"[^"]*""#).unwrap());

/// 문자열/식별자/주석을 공백으로 치환해서 분석 노이즈 제거.
fn strip_noise(sql: &str) -> String {
    let s = LINE_COMMENT.replace_all(sql, " ").into_owned();
    let s = BLOCK_COMMENT.replace_all(&s, " ").into_owned();
    let s = STR_LITERAL.replace_all(&s, " ").into_owned();
    IDENT_QUOTED.replace_all(&s, " ").into_owned()
}

/// 토큰 단위로 분리하면서 위험 키워드를 검색.
pub fn classify(sql: &str) -> SqlClassification {
    let cleaned = strip_noise(sql);
    let upper = cleaned.to_ascii_uppercase();

    if upper.trim().is_empty() {
        return SqlClassification {
            kind: SqlKind::Empty,
            keywords: vec![],
        };
    }

    // 단어 단위 토큰 모음
    let tokens: Vec<&str> = upper
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .collect();
    let token_set: std::collections::HashSet<&str> = tokens.iter().copied().collect();

    let write_keywords = ["INSERT", "UPDATE", "DELETE", "MERGE", "BULK"];
    let ddl_keywords = [
        "CREATE", "ALTER", "DROP", "TRUNCATE", "RENAME", "GRANT", "REVOKE", "DENY",
    ];

    let mut found_ddl: Vec<String> = ddl_keywords
        .iter()
        .filter(|k| token_set.contains(*k))
        .map(|k| k.to_string())
        .collect();
    let mut found_write: Vec<String> = write_keywords
        .iter()
        .filter(|k| token_set.contains(*k))
        .map(|k| k.to_string())
        .collect();

    if !found_ddl.is_empty() {
        found_ddl.append(&mut found_write);
        return SqlClassification {
            kind: SqlKind::Ddl,
            keywords: found_ddl,
        };
    }
    if !found_write.is_empty() {
        return SqlClassification {
            kind: SqlKind::Write,
            keywords: found_write,
        };
    }

    // EXEC / EXECUTE 는 어떤 SP 를 부르는지 모른다 — 일단 Unknown 으로
    // (sp_help 같은 read-only 도 많지만 보수적으로 Write 처럼 다이얼로그를 띄움)
    if token_set.contains("EXEC") || token_set.contains("EXECUTE") {
        return SqlClassification {
            kind: SqlKind::Unknown,
            keywords: vec!["EXEC".to_string()],
        };
    }

    // SELECT / WITH (CTE) / VALUES / DECLARE / USE / SET — read-only 로 본다
    let head = tokens.first().copied().unwrap_or("");
    let ro_heads = [
        "SELECT", "WITH", "VALUES", "DECLARE", "USE", "SET", "PRINT", "BEGIN", "GO",
    ];
    if ro_heads.contains(&head) {
        return SqlClassification {
            kind: SqlKind::ReadOnly,
            keywords: vec![],
        };
    }

    SqlClassification {
        kind: SqlKind::Unknown,
        keywords: vec![head.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_is_readonly() {
        assert_eq!(classify("SELECT 1").kind, SqlKind::ReadOnly);
    }

    #[test]
    fn insert_is_write() {
        assert_eq!(
            classify("INSERT INTO t VALUES (1)").kind,
            SqlKind::Write
        );
    }

    #[test]
    fn drop_is_ddl() {
        assert_eq!(classify("DROP TABLE foo").kind, SqlKind::Ddl);
    }

    #[test]
    fn ignores_string_literal() {
        // 문자열 안의 'DROP' 은 무시되어야 함
        assert_eq!(
            classify("SELECT 'DROP TABLE foo' AS x").kind,
            SqlKind::ReadOnly
        );
    }

    #[test]
    fn ignores_line_comment() {
        assert_eq!(
            classify("SELECT 1 -- DROP TABLE foo").kind,
            SqlKind::ReadOnly
        );
    }

    #[test]
    fn ignores_block_comment() {
        assert_eq!(
            classify("/* DROP */ SELECT 1").kind,
            SqlKind::ReadOnly
        );
    }

    #[test]
    fn ddl_wins_over_write() {
        let c = classify("INSERT INTO t VALUES (1); DROP TABLE t");
        assert_eq!(c.kind, SqlKind::Ddl);
    }

    #[test]
    fn empty_is_empty() {
        assert_eq!(classify("   ").kind, SqlKind::Empty);
        assert_eq!(classify("-- only a comment").kind, SqlKind::Empty);
    }

    #[test]
    fn exec_is_unknown() {
        assert_eq!(
            classify("EXEC sp_help 'sys.tables'").kind,
            SqlKind::Unknown
        );
    }
}
