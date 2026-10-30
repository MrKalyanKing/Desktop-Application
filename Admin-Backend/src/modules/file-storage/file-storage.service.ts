import { Injectable } from '@nestjs/common';
import { FileStorageProvider, StoredFile } from './file-storage.provider';

@Injectable()
export class FileStorageService {
  constructor(private readonly provider: FileStorageProvider) {}

  put(key: string, body: Buffer, mimeType: string): Promise<StoredFile> {
    return this.provider.put(key, body, mimeType);
  }

  get(key: string): Promise<Buffer> {
    return this.provider.get(key);
  }

  delete(key: string): Promise<void> {
    return this.provider.delete(key);
  }

  exists(key: string): Promise<boolean> {
    return this.provider.exists(key);
  }
}
