//! Turn a tool call into a plain-language step (en + vi) for the UI.
//! Texts are verb phrases without tense ("read x", "đọc x"); the UI adds it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

const RAW_ARG_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Step {
    pub icon: String,
    pub text_en: String,
    pub text_vi: String,
    /// `"{tool} {short arg}"`, for a tooltip.
    pub raw: String,
}

pub fn humanize(tool: &str, input: &Value) -> Step {
    let str_field = |k: &str| input.get(k).and_then(Value::as_str);
    let (icon, en, vi, arg): (&str, String, String, Option<&str>) = match tool {
        "Read" => {
            let fp = str_field("file_path");
            let (en, vi) = match fp.map(file_name) {
                Some(f) if !f.is_empty() => (format!("read {f}"), format!("đọc {f}")),
                _ => ("read a file".into(), "đọc file".into()),
            };
            ("📖", en, vi, fp)
        }
        "Edit" | "Write" | "MultiEdit" => {
            let fp = str_field("file_path");
            let (en, vi) = match fp.map(file_name) {
                Some(f) if !f.is_empty() => (format!("edit {f}"), format!("sửa {f}")),
                _ => ("edit a file".into(), "sửa file".into()),
            };
            ("✏️", en, vi, fp)
        }
        "Bash" => {
            let cmd = str_field("command");
            let (icon, en, vi) = bash_kind(cmd.unwrap_or(""));
            (icon, en.into(), vi.into(), cmd)
        }
        _ => {
            let (icon, en, vi) = match tool {
                "Grep" | "Glob" => ("🔍", "search the code".into(), "tìm kiếm trong code".into()),
                "WebFetch" | "WebSearch" => (
                    "🌐",
                    "look things up on the web".into(),
                    "tra cứu trên web".into(),
                ),
                "Task" | "Agent" => (
                    "🤝",
                    "hand work to an assistant".into(),
                    "giao việc cho trợ lý".into(),
                ),
                t if t.starts_with("mcp__plan-atlas__") => {
                    ("🗺", "update the plan".into(), "cập nhật plan".into())
                }
                t => ("⚙️", format!("use tool {t}"), format!("dùng công cụ {t}")),
            };
            let first = input
                .as_object()
                .and_then(|m| m.values().find_map(Value::as_str));
            (icon, en, vi, first)
        }
    };
    let raw = match arg {
        Some(a) if !a.is_empty() => {
            let short: String = a.chars().take(RAW_ARG_CHARS).collect();
            format!("{tool} {short}")
        }
        _ => tool.to_string(),
    };
    Step {
        icon: icon.into(),
        text_en: en,
        text_vi: vi,
        raw,
    }
}

/// Last path component, for both `/` and `\` separators.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn bash_kind(cmd: &str) -> (&'static str, &'static str, &'static str) {
    let lower = cmd.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    let any = |f: &dyn Fn(&str) -> bool| words.iter().any(|w| f(w));
    if any(&|w| matches!(w, "test" | "pytest" | "vitest" | "jest" | "nextest")) {
        ("🧪", "run tests", "chạy kiểm thử")
    } else if any(&|w| w.contains("lint") || w.contains("fmt") || w == "clippy") {
        ("🧹", "check code style", "kiểm tra quy chuẩn code")
    } else if words
        .iter()
        .enumerate()
        .any(|(i, w)| *w == "install" || (*w == "add" && (i == 0 || words[i - 1] != "git")))
    {
        ("📦", "install packages", "cài thư viện")
    } else {
        ("⚙️", "run a command", "chạy lệnh")
    }
}
