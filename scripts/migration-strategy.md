# 🚀 Database Migration Strategy: PocketBase → PostgreSQL + Redis

## Tổng quan
Migration từ PocketBase (SQLite-based) sang PostgreSQL + Redis Cluster để hỗ trợ 10,000+ CCU với performance cao và scalability tốt.

## Tại sao cần migration?
- **PocketBase limitations**: SQLite không scale được cho high-concurrency
- **Performance**: PostgreSQL xử lý được 1000+ connections đồng thời
- **Reliability**: ACID compliance, backup/restore mạnh mẽ
- **Advanced features**: Complex queries, stored procedures, triggers

## Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   PocketBase    │ -> │   Migration      │ -> │   PostgreSQL    │
│   (Source)      │    │   Scripts        │    │   (Target)      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │                        │
                              ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   Data           │    │   Redis         │
                       │   Transformation │    │   Cluster       │
                       └──────────────────┘    └─────────────────┘
```

## Migration Strategy

### Phase 1: Preparation (1-2 days)
- [ ] Setup PostgreSQL cluster (3 nodes minimum)
- [ ] Setup Redis cluster (3 nodes minimum)
- [ ] Backup current PocketBase data
- [ ] Test connection và permissions

### Phase 2: Schema Migration (1 day)
- [ ] Create PostgreSQL schema từ `database-schema.sql`
- [ ] Setup indexes và performance optimizations
- [ ] Test schema với sample data

### Phase 3: Data Migration (2-3 days)
- [ ] Export dữ liệu từ PocketBase theo batches
- [ ] Transform data để phù hợp với PostgreSQL schema
- [ ] Import data vào PostgreSQL với transaction safety
- [ ] Validate data integrity sau mỗi batch

### Phase 4: Application Migration (1-2 days)
- [ ] Update connection strings trong ứng dụng
- [ ] Implement Redis caching layer
- [ ] Test ứng dụng với new database
- [ ] Performance testing với load simulation

### Phase 5: Cutover & Validation (1 day)
- [ ] Zero-downtime migration (nếu có thể)
- [ ] Validate tất cả data đã được migrate
- [ ] Monitor performance và error rates
- [ ] Rollback plan nếu có vấn đề

## Data Mapping

### PocketBase → PostgreSQL

| PocketBase Collection | PostgreSQL Table | Notes |
|----------------------|------------------|-------|
| `users` | `players` | Merge với thông tin chi tiết hơn |
| `matches` | `games` | Map các trường phù hợp |
| `participants` | `game_sessions` | Liên kết với games và players |
| `leaderboard` | `players` (computed) | Tính từ player stats |
| `inventory` | `players` (extension) | Có thể cần bảng riêng nếu phức tạp |
| `achievements` | `players` (extension) | Có thể cần bảng riêng |
| `user_stats` | `player_stats` | Map trực tiếp |

### Redis Usage

| Use Case | Redis Pattern | TTL |
|----------|---------------|-----|
| Session Storage | `session:{token}` | 24h |
| User Profile Cache | `user:{id}:profile` | 1h |
| Leaderboard | `leaderboard:{season}:top` | 5m |
| Game State | `game:{id}:state` | Session |
| Rate Limiting | `ratelimit:{ip}:{action}` | 1m |
| Matchmaking Queue | `matchmaking:queue` | - |

## Risk Mitigation

### High Risk Areas
1. **Data Loss** - Implement comprehensive backup strategy
2. **Downtime** - Plan for zero-downtime nếu có thể
3. **Performance** - Load testing trước khi go-live

### Rollback Strategy
1. Keep PocketBase running during migration
2. Implement feature flags để switch giữa databases
3. Have automated rollback scripts ready

## Success Metrics

- [ ] 100% data integrity (row counts match)
- [ ] Performance improvement > 50%
- [ ] Zero data loss
- [ ] Migration time < 4 hours
- [ ] No production incidents during cutover

## Tools & Technologies

- **Migration Tools**: Custom PowerShell/Python scripts
- **Monitoring**: Prometheus + Grafana
- **Backup**: pg_dump, PocketBase backup API
- **Testing**: Load testing với 10,000+ simulated users
