# Production deployment

## Admin-Backend

1. Provision PostgreSQL 16+.
2. Set `TYPEORM_SYNC=false`.
3. Set strong JWT secrets and `CORS_ORIGINS` to the dashboard origin.
4. Set `STORAGE_PROVIDER=s3` with an IAM role (preferred) or keys only on the API server.
5. Run `npm run build && npm run migration:run && node dist/main`.
6. Put TLS in front of NestJS. The Rust desktop client must use HTTPS.

## Admin-Frontend

1. Build with `VITE_USE_MOCK=false` and `VITE_API_BASE=https://api.example.com/api/v1`.
2. Serve `dist/` from any static host.
3. Do not embed AWS or database secrets in the frontend bundle.

## Desktop

Ship KS Vision as today. Cloud sync is opt-in via Tauri commands. If NestJS is unreachable, local capture and Gemini continue and pending events remain in `cloud_sync_queue.json`.
