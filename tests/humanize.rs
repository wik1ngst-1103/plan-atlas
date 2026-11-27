use plan_atlas::events::humanize;
use serde_json::json;

fn vi(tool: &str, input: serde_json::Value) -> String {
    humanize(tool, &input).text_vi
}

#[test]
fn humanize_cases() {
    let read = humanize(
        "Read",
        &json!({"file_path": "/home/u/proj/src/auth/token.rs"}),
    );
    assert_eq!(read.icon, "📖");
    assert_eq!(read.text_vi, "đọc token.rs");
    assert_eq!(read.text_en, "read token.rs");
    assert_eq!(read.raw, "Read /home/u/proj/src/auth/token.rs");

    for tool in ["Edit", "Write", "MultiEdit"] {
        let s = humanize(tool, &json!({"file_path": "/p/web/App.svelte"}));
        assert_eq!(s.icon, "✏️");
        assert_eq!(s.text_vi, "sửa App.svelte");
        assert_eq!(s.text_en, "edit App.svelte");
    }

    for tool in ["Grep", "Glob"] {
        let s = humanize(tool, &json!({"pattern": "verify_token"}));
        assert_eq!(s.icon, "🔍");
        assert_eq!(s.text_vi, "tìm kiếm trong code");
        assert_eq!(s.text_en, "search the code");
    }

    let t = humanize("Bash", &json!({"command": "npm test -- token"}));
    assert_eq!(t.icon, "🧪");
    assert_eq!(t.text_vi, "chạy kiểm thử");
    assert_eq!(t.text_en, "run tests");
    assert_eq!(t.raw, "Bash npm test -- token");
    assert_eq!(
        vi("Bash", json!({"command": "cargo test --test hook"})),
        "chạy kiểm thử"
    );

    let l = humanize("Bash", &json!({"command": "cargo clippy --all-targets"}));
    assert_eq!(l.icon, "🧹");
    assert_eq!(l.text_vi, "kiểm tra quy chuẩn code");
    assert_eq!(l.text_en, "check code style");
    assert_eq!(
        vi("Bash", json!({"command": "cargo fmt --check"})),
        "kiểm tra quy chuẩn code"
    );
    assert_eq!(
        vi("Bash", json!({"command": "npm run lint"})),
        "kiểm tra quy chuẩn code"
    );

    let i = humanize("Bash", &json!({"command": "npm install zod"}));
    assert_eq!(i.icon, "📦");
    assert_eq!(i.text_vi, "cài thư viện");
    assert_eq!(i.text_en, "install packages");
    assert_eq!(
        vi("Bash", json!({"command": "cargo add serde"})),
        "cài thư viện"
    );
    // `git add` stages files; it does not install anything.
    assert_eq!(vi("Bash", json!({"command": "git add src/"})), "chạy lệnh");

    let b = humanize("Bash", &json!({"command": "ls -la"}));
    assert_eq!(b.icon, "⚙️");
    assert_eq!(b.text_vi, "chạy lệnh");
    assert_eq!(b.text_en, "run a command");

    for tool in ["WebFetch", "WebSearch"] {
        let s = humanize(tool, &json!({"url": "https://example.com"}));
        assert_eq!(s.icon, "🌐");
        assert_eq!(s.text_vi, "tra cứu trên web");
        assert_eq!(s.text_en, "look things up on the web");
    }

    for tool in ["Task", "Agent"] {
        let s = humanize(
            tool,
            &json!({"description": "find usages", "prompt": "..."}),
        );
        assert_eq!(s.icon, "🤝");
        assert_eq!(s.text_vi, "giao việc cho trợ lý");
        assert_eq!(s.text_en, "hand work to an assistant");
    }

    let p = humanize(
        "mcp__plan-atlas__start_task",
        &json!({"plan_id": "PLAN-x-001"}),
    );
    assert_eq!(p.icon, "🗺");
    assert_eq!(p.text_vi, "cập nhật plan");
    assert_eq!(p.text_en, "update the plan");

    let o = humanize("NotebookRead", &json!({"notebook_path": "a.ipynb"}));
    assert_eq!(o.icon, "⚙️");
    assert_eq!(o.text_vi, "dùng công cụ NotebookRead");
    assert_eq!(o.text_en, "use tool NotebookRead");
    assert_eq!(o.raw, "NotebookRead a.ipynb");
}

#[test]
fn raw_truncates_long_commands() {
    let long = "echo ".to_string() + &"é".repeat(500);
    let s = humanize("Bash", &json!({ "command": long }));
    assert_eq!(s.raw.chars().count(), "Bash ".len() + 200);
}

#[test]
fn odd_inputs_do_not_panic() {
    let s = humanize("Read", &json!(null));
    assert_eq!(s.icon, "📖");
    assert_eq!(s.raw, "Read");
    let s = humanize("Bash", &json!({"command": 42}));
    assert_eq!(s.text_en, "run a command");
}
