# scripts

Tien ich giup chay nhanh cac binary trong workspace.

## Native Deployment (Không cần Docker)

### Chạy từng service
- `run-service.ps1`: dung PowerShell (`pwsh -File scripts/run-service.ps1 gateway`).
- `run-service.sh`: dung shell Unix (`./scripts/run-service.sh gateway`).
  - Lan dau tren Unix can cap quyen: `chmod +x scripts/run-service.sh`.

### Development setup (tất cả services)
- `run-dev.ps1`: dung PowerShell (`pwsh -File scripts/run-dev.ps1`).
- `run-dev.sh`: dung shell Unix (`chmod +x scripts/run-dev.sh && ./scripts/run-dev.sh`).

### Production orchestrator
- `run-orchestrator.ps1`: khoi dong orchestrator voi file config (`pwsh -File scripts/run-orchestrator.ps1 -Config path/to/settings.json`).
- `run-orchestrator.sh`: tuong tu tren Unix (`chmod +x scripts/run-orchestrator.sh && ./scripts/run-orchestrator.sh`).

## PocketBase Integration

PocketBase được sử dụng thay thế Postgres + Redis, cung cấp:
- Database (SQLite/PostgreSQL)
- Authentication
- Real-time subscriptions
- Admin UI
- File storage

### Setup PocketBase

#### Windows (PowerShell)
```powershell
pwsh -File scripts/setup-pocketbase.ps1
```

#### Linux/Unix
```bash
chmod +x scripts/setup-pocketbase.sh
./scripts/setup-pocketbase.sh
```

Hoặc download manual:
```bash
# Download latest PocketBase binary
# Windows: https://github.com/pocketbase/pocketbase/releases/download/v0.22.0/pocketbase_0.22.0_windows_amd64.zip
# Linux: https://github.com/pocketbase/pocketbase/releases/download/v0.22.0/pocketbase_0.22.0_linux_amd64.zip
# Extract to pocketbase/ directory
```

### Services Architecture
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  PocketBase │◄──►│   Gateway   │◄──►│   Worker    │
│  (DB + Auth)│    │   (API)     │    │ (Game Logic)│
└─────────────┘    └─────────────┘    └─────────────┘
       │                    │
       ▼                    ▼
┌─────────────┐    ┌─────────────┐
│   Metrics   │    │   Logs      │
│ Prometheus  │    │   Tracing   │
└─────────────┘    └─────────────┘
```

Cac script nay don gian goi `cargo run` voi tham so phu hop de bo qua thao tac lap lai.
