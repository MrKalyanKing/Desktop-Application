export default () => ({
  nodeEnv: process.env.NODE_ENV ?? 'development',
  port: parseInt(process.env.PORT ?? '3001', 10),
  apiPrefix: 'api/v1',
  corsOrigins: (process.env.CORS_ORIGINS ?? 'http://localhost:5175,http://localhost:5174,http://localhost:5173')
    .split(',')
    .map((origin) => origin.trim()),
  jwt: {
    accessSecret: process.env.JWT_ACCESS_SECRET ?? 'dev-access-secret-change-me',
    refreshSecret: process.env.JWT_REFRESH_SECRET ?? 'dev-refresh-secret-change-me',
    accessTtl: process.env.JWT_ACCESS_TTL ?? '15m',
    refreshTtl: process.env.JWT_REFRESH_TTL ?? '7d',
  },
  database: {
    url: process.env.DATABASE_URL ?? 'postgres://parakeet:parakeet@localhost:5432/parakeet_admin',
    ssl: process.env.DATABASE_SSL === 'true',
    synchronize: process.env.TYPEORM_SYNC === 'true',
    logging: process.env.TYPEORM_LOGGING === 'true',
  },
  storage: {
    provider: (process.env.STORAGE_PROVIDER ?? 'local') as 'local' | 's3',
    localRoot: process.env.STORAGE_LOCAL_ROOT ?? 'storage',
    s3: {
      bucket: process.env.AWS_S3_BUCKET ?? '',
      region: process.env.AWS_REGION ?? 'us-east-1',
      accessKeyId: process.env.AWS_ACCESS_KEY_ID ?? '',
      secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY ?? '',
      endpoint: process.env.AWS_S3_ENDPOINT ?? undefined,
    },
  },
  uploads: {
    maxBytes: parseInt(process.env.UPLOAD_MAX_BYTES ?? `${10 * 1024 * 1024}`, 10),
    resumeMime: [
      'application/pdf',
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    ],
  },
  retention: {
    transcriptDays: parseInt(process.env.TRANSCRIPT_RETENTION_DAYS ?? '365', 10),
  },
  throttle: {
    ttl: parseInt(process.env.THROTTLE_TTL_MS ?? '60000', 10),
    limit: parseInt(process.env.THROTTLE_LIMIT ?? '120', 10),
  },
});
