---
id: PLAN-auth-gateway-001
project: auth-gateway
title: Migrate sessions to JWT + RBAC
tags: [type/feature, domain/security, tech/jwt]
related:
  - { to: PLAN-auth-gateway-000, type: depends_on }
created: 2026-11-20
---
## 01. Overview

Move sessions to JWT.

## 02. Requirements & Scope

Scope text.

## 03. Technical Analysis

### Current Architecture

Cookies.

### Existing Problems

Slow.

### Proposed Architecture

JWT.

## 04. Task Breakdown

### TASK-001: Authentication Service
- complexity: 4
- model: opus
- effort: medium
- depends_on: []
- status: todo
- tags: [tech/jwt]

Free markdown description.

### TASK-002: Token refresh
- complexity: 4
- depends_on: [TASK-001]
- status: in_progress

Refresh flow.

### TASK-003: RBAC middleware
- complexity: 2

Roles.

## 05. Risks & Rollback

Rollback plan.
