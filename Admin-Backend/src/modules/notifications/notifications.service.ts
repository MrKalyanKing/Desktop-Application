import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { IsNull, Repository } from 'typeorm';
import { Notification } from './notification.entity';

@Injectable()
export class NotificationsService {
  constructor(
    @InjectRepository(Notification) private readonly notifications: Repository<Notification>,
  ) {}

  list(userId: string) {
    return this.notifications.find({ where: { userId }, order: { createdAt: 'DESC' }, take: 50 });
  }

  async markRead(userId: string, id: string) {
    const row = await this.notifications.findOne({ where: { id, userId } });
    if (!row) return null;
    row.readAt = new Date();
    return this.notifications.save(row);
  }

  unreadCount(userId: string) {
    return this.notifications.count({ where: { userId, readAt: IsNull() } });
  }
}
