# 🗺 Plan Atlas

[English](README.md) · **Tiếng Việt**

**Một nơi duy nhất cho mọi plan Claude Code tạo ra — lập trình viên hay người không chuyên đều đọc hiểu được.**

Plan Atlas là ứng dụng mã nguồn mở, nhỏ gọn, chạy ngay trên máy bạn. Nó gom các plan triển khai mà Claude Code tạo ở mọi dự án, hiển thị chúng như tài liệu gọn gàng kèm bảng công việc, và cho bạn xem Claude cùng các trợ lý (subagent) đang làm gì — bằng câu chữ dễ hiểu, theo thời gian thực.

> Trạng thái: **pre-alpha**. Bản mẫu giao diện ở [`docs/prototype/index.html`](docs/prototype/index.html); plan triển khai v1 ở [`docs/superpowers/plans/`](docs/superpowers/plans/).

---

## Vì sao cần

Claude Code lưu plan vào `~/.claude/plans/` với tên ngẫu nhiên kiểu `jiggly-popping-popcorn.md`. Vài tuần sau bạn có hàng chục file, không biết file nào của dự án nào, cũng không xem được tiến độ. Plan Atlas giải quyết việc đó:

| | |
|---|---|
| 📚 **Một kho, chia theo dự án** | Mọi plan nằm trong `~/.plan-atlas/plans/<dự-án>/`. Plan cũ của Claude Code được tự động nhập (file gốc không bao giờ bị sửa). |
| 📄 **Plan như tài liệu** | Cấu trúc cố định — Overview, Requirements & Scope, Technical Analysis, Task Breakdown, Risks — kèm mục lục “Trên trang này”. |
| ✅ **Theo dõi công việc** | Mỗi việc có trạng thái, độ khó (1–5) và model + mức suy nghĩ được khuyến nghị (Haiku / Sonnet / Opus). Kéo thẻ trên bảng để cập nhật. |
| 🤖 **Hoạt động trực tiếp** | Thấy “Trợ lý lập trình đang sửa phần lưu token” lồng ngay dưới đúng plan và việc. Lệnh gốc chỉ cách một cú bấm, dành cho lập trình viên. |
| 🧭 **Bản đồ kiến thức** | Chủ đề (tag) theo bộ phân loại chung, cùng đồ thị quan hệ giữa các plan (`depends_on`, `extends`, `relates_to`, cùng chủ đề). |
| 🔍 **Tìm kiếm** | Plan, việc và chủ đề. Gõ không dấu vẫn ra (`dang nhap` → `đăng nhập`). |
| 🌐 **Song ngữ** | Giao diện tiếng Anh và tiếng Việt, tự chọn theo trình duyệt, đổi được bằng nút trên thanh trên cùng. |

## Cách hoạt động

```
Claude Code ──(MCP, stdio)── plan-atlas mcp ──┐
Claude Code ──(hooks)─────── plan-atlas hook ─┼──► chỉ mục SQLite ◄── plan-atlas (web, 127.0.0.1)
~/.plan-atlas/plans/*.md ◄──── theo dõi file ─┘                          ▲
                                                                   trình duyệt của bạn
```

- **Một file chạy duy nhất**, viết bằng Rust: web server, MCP server, xử lý hook và trình cài đặt gói trong một file khoảng 9 MB. Mục tiêu RAM khi rảnh: **≤ 40 MB** với 1.000 plan.
- **File Markdown là nguồn sự thật.** SQLite chỉ là chỉ mục — xoá đi sẽ tự dựng lại.
- **Chỉ chạy cục bộ.** Server chỉ nghe ở `127.0.0.1`, không gửi dữ liệu đi đâu.

## Cài đặt

> Bản phát hành đầu tiên (v0.1.0) đang chờ: các lệnh dưới đây sẽ dùng được sau khi phát hành.

**macOS / Linux**
```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/WiK1ngST/plan-atlas/releases/latest/download/plan-atlas-installer.sh | sh
```

**Windows (PowerShell)**
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/WiK1ngST/plan-atlas/releases/latest/download/plan-atlas-installer.ps1 | iex"
```

**Cách khác**
```sh
brew install WiK1ngST/tap/plan-atlas   # Homebrew
npx plan-atlas                         # npm, không cần cài toàn cục
```

Từ mã nguồn (cần Rust 1.89+ và Node 22): `cd web && npm ci && npm run build && cd .. && cargo build --release` — giao diện web được nhúng vào file chạy, xem [CONTRIBUTING.md](CONTRIBUTING.md).

## Sử dụng

```sh
plan-atlas
```

Vậy thôi. Lần đầu chạy, Plan Atlas hỏi có kết nối với Claude Code không, nhập các plan sẵn có rồi mở trình duyệt tại `http://127.0.0.1:7420`. Chạy lại khi đang chạy thì chỉ mở thêm một tab.

| Lệnh | Tác dụng |
|---|---|
| `plan-atlas` | Chạy ứng dụng và mở trình duyệt (`--port <n>`, `--no-open`) |
| `plan-atlas setup` | Cài Skill, hooks và MCP server vào Claude Code |
| `plan-atlas uninstall-claude` | Gỡ mọi thứ `setup` đã thêm (giữ nguyên cài đặt khác của bạn) |
| `plan-atlas import` | Nhập lại plan từ `~/.claude/plans/` |
| `plan-atlas mcp` | MCP server qua stdio (Claude Code tự gọi) |
| `plan-atlas hook` | Xử lý hook (Claude Code tự gọi) |

### `setup` thay đổi những gì

- Thêm `~/.claude/skills/plan-atlas/SKILL.md` để Claude biết tạo plan ở đâu và theo cấu trúc nào.
- Thêm hook cho `PreToolUse`, `SubagentStart`, `SubagentStop`, `SessionEnd` vào `~/.claude/settings.json` — có bản sao lưu kèm thời gian, hook sẵn có được giữ nguyên.
- Đăng ký MCP server: `claude mcp add --scope user plan-atlas -- plan-atlas mcp`.

### Cấu hình

| | Mặc định | Thay đổi bằng |
|---|---|---|
| Thư mục dữ liệu | `~/.plan-atlas` | `PLAN_ATLAS_HOME` |
| Cổng | `7420` (tự lùi tới `7430`) | `--port`, `PLAN_ATLAS_PORT` |
| Ngôn ngữ giao diện | Theo trình duyệt (English hoặc Tiếng Việt) | nút đổi ngôn ngữ trên thanh trên cùng |
| Chủ đề | `~/.plan-atlas/taxonomy.toml` | sửa file rồi chạy `plan-atlas setup` để cập nhật Skill |

## Định dạng plan

Xem ví dụ đầy đủ trong [README tiếng Anh](README.md#plan-format). Bạn hiếm khi phải tự viết: Claude tạo plan qua tool MCP `create_plan`. Sửa tay vẫn được — file được lập chỉ mục lại khi lưu, lỗi hiện thành cảnh báo trong giao diện chứ không làm hỏng ứng dụng.

## Quyền riêng tư & bảo mật

- Không thu thập dữ liệu sử dụng, không gọi mạng ngoài `127.0.0.1`.
- Request ghi dữ liệu bắt buộc có header riêng và `Host` là localhost (chống DNS rebinding).
- Hook không bao giờ chặn Claude Code: luôn kết thúc trong 200 ms với mã thoát 0.

Báo lỗ hổng bảo mật theo hướng dẫn trong [SECURITY.md](SECURITY.md).

## Đóng góp

Rất hoan nghênh đóng góp — code, bản dịch, chủ đề cho bộ phân loại, tài liệu, báo lỗi. Bắt đầu từ [CONTRIBUTING.md](CONTRIBUTING.md) (tiếng Anh). Issue và pull request viết bằng tiếng Việt cũng được chấp nhận. Mọi người tham gia đều đồng ý với [Quy tắc ứng xử](CODE_OF_CONDUCT.md).

## Lộ trình

- [ ] v0.1 — kho plan, nhập plan cũ, tài liệu, bảng công việc, hoạt động trực tiếp, bản đồ kiến thức, tìm kiếm, bộ cài
- [ ] Chạy nền như dịch vụ hệ điều hành (launchd / systemd / Windows service)
- [ ] Chi phí & token theo từng việc
- [ ] Xuất plan thành trang tĩnh để chia sẻ
- [ ] Thêm ngôn ngữ giao diện

## Giấy phép

[MIT](LICENSE) © Plan Atlas contributors
