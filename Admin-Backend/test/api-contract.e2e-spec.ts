import { INestApplication, ValidationPipe } from '@nestjs/common';
import { Test } from '@nestjs/testing';
import request from 'supertest';
import { AppModule } from '../src/app.module';

const db = process.env.DATABASE_URL;

describe('API contract (optional live postgres)', () => {
  let app: INestApplication;

  beforeAll(async () => {
    if (!db) return;
    const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
    app = moduleRef.createNestApplication();
    app.setGlobalPrefix('api/v1');
    app.useGlobalPipes(new ValidationPipe({ whitelist: true, transform: true }));
    await app.init();
  });

  afterAll(async () => {
    if (app) await app.close();
  });

  it('health is public', async () => {
    if (!db) return;
    await request(app.getHttpServer()).get('/api/v1/health').expect(200);
  });

  it('protected routes require a bearer token', async () => {
    if (!db) return;
    await request(app.getHttpServer()).get('/api/v1/interview-sessions').expect(401);
  });
});
