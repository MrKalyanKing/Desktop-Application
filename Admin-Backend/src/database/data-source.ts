import { DataSource } from 'typeorm';
import { config as loadEnv } from 'dotenv';
import { ALL_ENTITIES } from './entities';

loadEnv();

export default new DataSource({
  type: 'postgres',
  url: process.env.DATABASE_URL,
  ssl: process.env.DATABASE_SSL === 'true' ? { rejectUnauthorized: false } : false,
  entities: ALL_ENTITIES,
  migrations: ['src/database/migrations/*.ts'],
  logging: process.env.TYPEORM_LOGGING === 'true',
});
