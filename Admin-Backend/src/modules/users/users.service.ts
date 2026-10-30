import { Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { UserRole, UserStatus } from '../../common/enums';
import { User } from './user.entity';

@Injectable()
export class UsersService {
  constructor(@InjectRepository(User) private readonly users: Repository<User>) {}

  list(organizationId: string) {
    return this.users.find({
      where: { organizationId },
      select: ['id', 'email', 'firstName', 'lastName', 'role', 'status', 'lastLoginAt', 'createdAt'],
      order: { createdAt: 'DESC' },
    });
  }

  async update(
    organizationId: string,
    id: string,
    patch: Partial<Pick<User, 'firstName' | 'lastName' | 'role' | 'status'>>,
  ) {
    const user = await this.users.findOne({ where: { id, organizationId } });
    if (!user) throw new NotFoundException('User not found');
    Object.assign(user, patch);
    return this.users.save(user);
  }

  async invite(
    organizationId: string,
    data: { email: string; firstName: string; lastName: string; role: UserRole },
  ) {
    const user = this.users.create({
      organizationId,
      email: data.email.toLowerCase(),
      firstName: data.firstName,
      lastName: data.lastName,
      role: data.role,
      status: UserStatus.INVITED,
      passwordHash: '',
    });
    return this.users.save(user);
  }
}
