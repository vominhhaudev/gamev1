# scripts

Tien ich giup chay nhanh cac binary trong workspace.
- `run-service.ps1`: dung PowerShell (`pwsh -File scripts/run-service.ps1 gateway`). Co the truyen tiep tham so sau ten service.
- `run-service.sh`: dung shell Unix (`./scripts/run-service.sh gateway`). Lan dau tren Unix can cap quyen: `chmod +x scripts/run-service.sh`.
- `run-orchestrator.ps1`: khoi dong orchestrator voi file config (`pwsh -File scripts/run-orchestrator.ps1 -Config path/to/settings.json`).
- `run-orchestrator.sh`: tuong tu tren Unix (`chmod +x scripts/run-orchestrator.sh && ./scripts/run-orchestrator.sh`).

Cac script nay don gian goi `cargo run` voi tham so phu hop de bo qua thao tac lap lai.
