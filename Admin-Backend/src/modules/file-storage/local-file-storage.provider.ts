import { Injectable } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { mkdir, readFile, unlink, writeFile, access } from 'fs/promises';
import { dirname, join } from 'path';
import { FileStorageProvider, StoredFile } from './file-storage.provider';

@Injectable()
export class LocalFileStorageProvider extends FileStorageProvider {
  private readonly root: string;

  constructor(config: ConfigService) {
    super();
    this.root = config.get<string>('storage.localRoot') ?? 'storage';
  }

  private resolve(key: string) {
    const safe = key.replace(/\.\./g, '');
    return join(this.root, safe);
  }

  async put(key: string, body: Buffer, mimeType: string): Promise<StoredFile> {
    const path = this.resolve(key);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, body);
    return { key, provider: 'local', sizeBytes: body.length, mimeType };
  }

  async get(key: string): Promise<Buffer> {
    return readFile(this.resolve(key));
  }

  async delete(key: string): Promise<void> {
    await unlink(this.resolve(key)).catch(() => undefined);
  }

  async exists(key: string): Promise<boolean> {
    try {
      await access(this.resolve(key));
      return true;
    } catch {
      return false;
    }
  }
}
