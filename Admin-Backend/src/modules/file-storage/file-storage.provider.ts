export interface StoredFile {
  key: string;
  provider: 'local' | 's3';
  sizeBytes: number;
  mimeType: string;
}

export abstract class FileStorageProvider {
  abstract put(key: string, body: Buffer, mimeType: string): Promise<StoredFile>;
  abstract get(key: string): Promise<Buffer>;
  abstract delete(key: string): Promise<void>;
  abstract exists(key: string): Promise<boolean>;
}
