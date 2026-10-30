import { Module } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { FileStorageProvider } from './file-storage.provider';
import { FileStorageService } from './file-storage.service';
import { LocalFileStorageProvider } from './local-file-storage.provider';
import { S3FileStorageProvider } from './s3-file-storage.provider';

@Module({
  providers: [
    {
      provide: FileStorageProvider,
      inject: [ConfigService],
      useFactory: (config: ConfigService) => {
        const provider = config.get<'local' | 's3'>('storage.provider');
        return provider === 's3'
          ? new S3FileStorageProvider(config)
          : new LocalFileStorageProvider(config);
      },
    },
    FileStorageService,
  ],
  exports: [FileStorageService],
})
export class FileStorageModule {}
