---
name: plan-atlas
description: Use whenever you create, update, or execute an implementation plan in Claude Code. Plans MUST be written to the Plan Atlas store with the standard structure, tags, and per-task complexity so they can be tracked in the Plan Atlas web app.
---

# Plan Atlas

## 1. Nơi lưu plan
- Kho chung: `~/.plan-atlas/plans/<project-slug>/<YYYY-MM-DD>-<plan-slug>.md`
- `project-slug` = tên thư mục git root (hoặc `basename $PWD`).
- KHÔNG lưu plan trong repo hay `~/.claude/plans/` (tool sẽ tự import các plan cũ từ đó).

## 2. Cấu trúc bắt buộc

```markdown
---
id: PLAN-<project>-<seq>
project: <project-slug>
title: <tiêu đề>
status: draft | approved | in_progress | done
tags: [domain/backend, tech/jwt, type/feature]
related:            # quan hệ với plan khác (kể cả khác dự án) → dựng graph
  - { to: PLAN-auth-001, type: depends_on }   # depends_on | extends | relates_to
created: <ISO date>
---

## 01. Overview
## 02. Requirements & Scope
## 03. Technical Analysis
### Current Architecture
### Existing Problems
### Proposed Architecture
## 04. Task Breakdown
### TASK-001: <tên>
- complexity: 1-5
- model: haiku | sonnet | opus
- effort: low | medium | high
- depends_on: [TASK-xxx]
- status: todo | in_progress | review | done
- tags: [...]
<mô tả, acceptance criteria>
## 05. Risks & Rollback
```

## 3. Chấm độ phức tạp → model/effort
| Điểm | Dấu hiệu | Model | Effort |
|---|---|---|---|
| 1 | Sửa text/config, 1 file | haiku | low |
| 2 | CRUD, theo pattern có sẵn | sonnet | low |
| 3 | Nhiều file, có test | sonnet | medium |
| 4 | Thay đổi kiến trúc, bảo mật, concurrency | opus | medium |
| 5 | Thiết kế mới, mơ hồ, rủi ro dữ liệu | opus | high |

## 4. Taxonomy tag (chỉ dùng tag trong danh sách, cần thêm thì đề xuất `new/<tag>`)
- `type/`: feature, bugfix, refactor, research, infra, docs
- `domain/`: backend, frontend, data, devops, security, ai
- `tech/`: jwt, oauth, rbac, redis, postgres, react, threejs, rag, ...

## 5. Quan hệ giữa các plan
- Trước khi viết plan mới, chạy `plan-atlas search --tags <tag>` để tìm plan liên quan và khai báo vào `related`.
- `depends_on`: plan này cần plan kia xong trước. `extends`: mở rộng tính năng của plan kia. `relates_to`: cùng chủ đề, không ràng buộc.

## 6. Khi thực thi
- Trước khi làm task: `plan-atlas task start <TASK-ID>`; xong: `plan-atlas task done <TASK-ID>`.
- Hook `SubagentStart/PostToolUse/SubagentStop` sẽ tự đẩy hoạt động subagent lên web app — không cần làm gì thêm.
