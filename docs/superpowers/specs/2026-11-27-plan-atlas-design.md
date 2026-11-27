# Plan Atlas — Design Spec

Ngày: 2026-11-27 · Trạng thái: chờ duyệt · Prototype UI: `docs/prototype/index.html`

## 1. Sản phẩm là gì

Ứng dụng mã nguồn mở chạy **cục bộ trên máy cá nhân**, mở bằng trình duyệt, để:

1. Gom mọi plan Claude Code tạo ra (ở mọi dự án) về **một kho duy nhất**, phân theo dự án.
2. Cài **Skill + MCP server + hooks** vào Claude Code để Claude biết tạo plan ở đâu, theo cấu trúc nào.
3. Hiển thị plan như **tài liệu có mục lục cây** (01 Overview … 05 Risks), mỗi task có **độ phức tạp 1–5 → model + effort**, trạng thái, đánh dấu hoàn thành.
4. Hiển thị **Claude/subagent đang làm gì theo thời gian thực**, bằng ngôn ngữ thường, lồng dưới plan → task.
5. **Chủ đề (tag)** theo bộ phân loại cố định → cây kiến thức + **bản đồ quan hệ giữa các plan** (khai báo `related` + suy ra từ chủ đề chung).
6. **Tìm kiếm** plan / việc / chủ đề, gõ không dấu được.

Đối tượng: lập trình viên dùng Claude Code **và** người không chuyên (PM, khách hàng) xem tiến độ.

## 2. Nguyên tắc UX (đã chốt qua prototype)

- Phân cấp rõ **Dự án → Plan → Việc**, luôn có breadcrumb.
- Bấm việc/chủ đề → **mở panel bên phải**, không bao giờ nhảy trang. Chỉ hành động điều hướng rõ ràng (“Mở plan ›”) mới đổi trang. Nút Back trình duyệt hoạt động.
- Hoạt động của Claude luôn **lồng dưới plan và việc của nó**. Câu chữ thường (“Trợ lý lập trình đang sửa phần lưu token”), lệnh gốc ẩn sau “Hiện chi tiết kỹ thuật”.
- Tag hiển thị bằng nhãn thường (“Phân quyền theo vai trò”), không phải `tech/rbac`.
- Giao diện đa ngôn ngữ: **en** (mặc định), **vi**.

## 3. Kiến trúc

```
Claude Code ──(stdio)── plan-atlas mcp ─┐            ┌── Trình duyệt (SPA nhúng trong binary)
Claude Code ──(hook)─── plan-atlas hook ─┼─► SQLite ◄─┤        ▲ REST + SSE
~/.plan-atlas/plans/*.md ◄── watcher ────┘  (WAL)    └── plan-atlas (axum, 127.0.0.1)
```

- **Một binary duy nhất `plan-atlas`** (Rust) với các lệnh con: (mặc định) chạy server + mở trình duyệt; `mcp`; `hook`; `setup`; `uninstall-claude`; `import`; `task`.
- **Nguồn sự thật cho plan = file Markdown** trong `~/.plan-atlas/plans/<project>/`. SQLite chỉ là chỉ mục (xoá đi dựng lại được). Đổi trạng thái task trên web → ghi lại đúng dòng `- status:` trong file.
- **Sự kiện hoạt động** chỉ lưu trong SQLite (bảng `events`, tự dọn sau 7 ngày). Hook ghi thẳng vào SQLite nên **không mất sự kiện khi web chưa chạy**. Server phát hiện thay đổi bằng `PRAGMA data_version` mỗi 500 ms rồi đẩy SSE.
- **Liên kết hoạt động ↔ task**: khi Claude gọi tool MCP `start_task`, hook `PreToolUse` thấy `tool_name = mcp__plan-atlas__start_task` cùng `session_id` → ghi bảng `sessions(session_id → plan_id, task_id)`. Mọi sự kiện sau đó của session gắn vào task đó.

## 4. Vì sao Rust

Ứng dụng chạy nền trên máy cá nhân → ưu tiên RAM/CPU thấp, một file chạy, không cần runtime:

| | Rust | Go | Node/Electron |
|---|---|---|---|
| RAM khi rảnh | ~10–25 MB | ~20–40 MB (GC) | 80–300 MB |
| Phân phối | 1 binary tĩnh | 1 binary | cần runtime / Chromium |
| Bộ nhớ | không GC, kiểm soát chính xác | GC | GC |

Frontend: **Svelte 5 + Vite + TypeScript** (biên dịch ra JS nhỏ, không runtime ảo DOM), build một lần rồi **nhúng vào binary** bằng `rust-embed`. Người dùng cuối không cần Node.

## 5. Định dạng plan

```markdown
---
id: PLAN-auth-gateway-001
project: auth-gateway
title: Migrate sessions to JWT + RBAC
tags: [type/feature, domain/security, tech/jwt]
related:
  - { to: PLAN-auth-gateway-000, type: depends_on }   # depends_on | extends | relates_to
created: 2026-11-20
---
## 01. Overview
## 02. Requirements & Scope
## 03. Technical Analysis
### Current Architecture
### Existing Problems
### Proposed Architecture
## 04. Task Breakdown
### TASK-001: Authentication Service
- complexity: 4
- model: opus
- effort: medium
- depends_on: []
- status: todo
- tags: [tech/jwt]

Free markdown description / acceptance criteria.
## 05. Risks & Rollback
```

Độ phức tạp → model/effort mặc định (khi thiếu): 1→haiku/low · 2→sonnet/low · 3→sonnet/medium · 4→opus/medium · 5→opus/high.

Plan nhập từ `~/.claude/plans/` (không có cấu trúc) → `structured = false`, vẫn hiển thị theo heading, đoán dự án bằng cách tìm tên file plan trong transcript `~/.claude/projects/*/*.jsonl`; không đoán được → dự án `unsorted`. **Không sửa/xoá file gốc.**

## 6. Bộ phân loại chủ đề

`~/.plan-atlas/taxonomy.toml`, 3 nhóm `type/`, `domain/`, `tech/`, mỗi tag có nhãn + mô tả en/vi. `setup` sinh phần tag trong SKILL.md từ file này. Tag ngoài danh sách → cảnh báo trong UI, Claude được hướng dẫn đề xuất `new/<tag>`.

## 7. Cài đặt & chạy

- Phát hành bằng **cargo-dist**: script cài 1 dòng (macOS/Linux `curl … | sh`, Windows PowerShell), Homebrew, npm (`npx plan-atlas`), `cargo install plan-atlas`.
- Người dùng gõ `plan-atlas` → (lần đầu hỏi cài vào Claude Code) → nhập plan cũ → chạy server → **tự mở trình duyệt**. Lần chạy thứ hai khi đang chạy → chỉ mở trình duyệt.

## 8. Ngân sách tài nguyên & an toàn

- RAM rảnh ≤ 40 MB với 1.000 plan; CPU rảnh ≈ 0; binary ≤ 20 MB; JS gzip ≤ 200 KB; từ lúc gõ lệnh đến mở trình duyệt ≤ 1 s.
- Chỉ lắng nghe `127.0.0.1`; kiểm tra header `Host` (chống DNS rebinding); request ghi bắt buộc header `X-Plan-Atlas: 1`; không bật CORS.
- `hook` không bao giờ làm hỏng Claude Code: luôn thoát mã 0 trong ≤ 200 ms, không in ra stdout.

## 9. Ngoài phạm vi v1

Đồng bộ nhiều máy / cloud, tài khoản người dùng, tự chạy nền như service hệ điều hành, tự động điều phối model cho subagent (v1 chỉ ghi và hiển thị khuyến nghị; Skill hướng dẫn Claude dùng đúng model).
