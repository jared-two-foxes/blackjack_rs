# AGENTS.md

## Repository Overview

This repository contains the core logic for a Blackjack game implemented in Rust. The main source files are located in the `src/` directory, and the project is managed using Cargo. The codebase is modular, with separate files for data sources, utility functions, and type definitions.

## Project Task Management System

This project integrates with a task management API, which can be accessed at `http://127.0.0.1:8010`. The API provides endpoints for managing projects and tasks, allowing for streamlined project tracking and automation.

### API Overview

- **Base URL:** `http://127.0.0.1:8010`
- **OpenAPI Spec:** [openapi.json](http://127.0.0.1:8010/openapi.json)

#### Project Endpoints
- `GET /projects` — List all projects
- `POST /projects` — Create a new project (requires project name in JSON body)
- `DELETE /projects/{name}` — Delete a project by name

#### Task Endpoints
- `GET /tasks?project=<project_id>` — List all tasks for a project
- `POST /tasks` — Create a new task (requires JSON body with task details)
- `GET /tasks/recommended?project=<project_id>` — Get recommended tasks for a project
- `GET /tasks/sync?project=<project_id>&since=<timestamp>` — List tasks updated since a given time
- `POST /tasks/sync` — Upsert (create/update) multiple tasks in bulk
- `GET /tasks/{task_id}` — Get details for a specific task
- `PATCH /tasks/{task_id}` — Update a specific task

#### Task Object Schema
A task object includes:
- `id` (UUID)
- `project_id` (string)
- `title` (string)
- `description` (string)
- `status` (string)
- `assignee` (string, nullable)
- `parent_id` (UUID, nullable)
- `created_at` (string, timestamp)
- `updated_at` (string, timestamp)

#### Status Endpoints
- `GET /statuses` — List all possible task statuses

### Example: Creating a Task
```bash
curl -X POST http://127.0.0.1:8010/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "project_id": "my_project",
    "title": "Implement game logic",
    "description": "Write core Blackjack logic in Rust",
    "status": "open"
  }'
```

### Example: Listing Tasks for a Project
```bash
curl http://127.0.0.1:8010/tasks?project=my_project
```

---

For more details, refer to the [OpenAPI documentation](http://127.0.0.1:8010/openapi.json) or contact the project maintainers.

---

## Code Style and Best Practices

All code in this repository should be arranged and formatted according to Rust best practices:

- Use `rustfmt` to automatically format code.
- Organize modules, imports, and functions clearly and idiomatically.
- Prefer explicitness, safety, and clarity in all code.
- Follow community conventions for naming, error handling, and documentation.
- Review and refactor code regularly to maintain quality and consistency.
