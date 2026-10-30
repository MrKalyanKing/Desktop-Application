import { createHash, randomBytes } from 'crypto';
import {
  ConflictException,
  Injectable,
  UnauthorizedException,
} from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { JwtService } from '@nestjs/jwt';
import { InjectRepository } from '@nestjs/typeorm';
import * as bcrypt from 'bcryptjs';
import { Repository } from 'typeorm';
import { DeviceStatus, UserRole, UserStatus } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { Organization } from '../organizations/organization.entity';
import { User } from '../users/user.entity';
import { DesktopDevice } from '../desktop-integration/desktop-device.entity';
import { RefreshToken } from './refresh-token.entity';
import { DesktopRegisterDto, LoginDto, RegisterDto } from './dto/auth.dto';

@Injectable()
export class AuthService {
  constructor(
    @InjectRepository(User) private readonly users: Repository<User>,
    @InjectRepository(Organization) private readonly orgs: Repository<Organization>,
    @InjectRepository(RefreshToken) private readonly refreshTokens: Repository<RefreshToken>,
    @InjectRepository(DesktopDevice) private readonly devices: Repository<DesktopDevice>,
    private readonly jwt: JwtService,
    private readonly config: ConfigService,
  ) {}

  async register(dto: RegisterDto) {
    const existing = await this.users.findOne({ where: { email: dto.email.toLowerCase() } });
    if (existing) throw new ConflictException('Email already registered');

    const slug = dto.organizationName
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .concat('-', randomBytes(3).toString('hex'));

    const organization = await this.orgs.save(
      this.orgs.create({ name: dto.organizationName, slug, settings: {} }),
    );

    const user = await this.users.save(
      this.users.create({
        organizationId: organization.id,
        email: dto.email.toLowerCase(),
        passwordHash: await bcrypt.hash(dto.password, 12),
        firstName: dto.firstName,
        lastName: dto.lastName,
        role: UserRole.OWNER,
        status: UserStatus.ACTIVE,
      }),
    );

    return this.issueTokens(user);
  }

  async registerCustomer(dto: { email: string; password: string; firstName: string; lastName: string }) {
    const existing = await this.users.findOne({ where: { email: dto.email.toLowerCase() } });
    if (existing) throw new ConflictException('Email already registered');

    const slug = `${dto.firstName}-${dto.lastName}`
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .concat('-', randomBytes(3).toString('hex'));

    const organization = await this.orgs.save(
      this.orgs.create({
        name: `${dto.firstName} ${dto.lastName}`,
        slug,
        settings: { workspace: 'customer' },
      }),
    );

    const user = await this.users.save(
      this.users.create({
        organizationId: organization.id,
        email: dto.email.toLowerCase(),
        passwordHash: await bcrypt.hash(dto.password, 12),
        firstName: dto.firstName,
        lastName: dto.lastName,
        role: UserRole.CANDIDATE,
        status: UserStatus.ACTIVE,
      }),
    );

    return this.issueTokens(user);
  }

  async login(dto: LoginDto) {
    const user = await this.users.findOne({ where: { email: dto.email.toLowerCase() } });
    if (!user || user.status !== UserStatus.ACTIVE) {
      throw new UnauthorizedException('Invalid credentials');
    }
    const ok = await bcrypt.compare(dto.password, user.passwordHash);
    if (!ok) throw new UnauthorizedException('Invalid credentials');
    user.lastLoginAt = new Date();
    await this.users.save(user);
    return this.issueTokens(user);
  }

  async refresh(refreshToken: string) {
    const tokenHash = this.hash(refreshToken);
    const stored = await this.refreshTokens.findOne({
      where: { tokenHash },
      relations: ['user'],
    });
    if (!stored || stored.revokedAt || stored.expiresAt < new Date()) {
      throw new UnauthorizedException('Invalid refresh token');
    }
    stored.revokedAt = new Date();
    await this.refreshTokens.save(stored);
    return this.issueTokens(stored.user);
  }

  async logout(refreshToken: string) {
    const stored = await this.refreshTokens.findOne({ where: { tokenHash: this.hash(refreshToken) } });
    if (stored && !stored.revokedAt) {
      stored.revokedAt = new Date();
      await this.refreshTokens.save(stored);
    }
    return { success: true };
  }

  async me(user: AuthUser) {
    const record = await this.users.findOne({
      where: { id: user.id },
      relations: ['organization'],
    });
    if (!record) throw new UnauthorizedException();
    return {
      id: record.id,
      email: record.email,
      firstName: record.firstName,
      lastName: record.lastName,
      role: record.role,
      organization: {
        id: record.organization.id,
        name: record.organization.name,
        slug: record.organization.slug,
      },
    };
  }

  async registerDesktop(dto: DesktopRegisterDto) {
    const tokens = await this.login({ email: dto.email, password: dto.password });
    const user = await this.users.findOneByOrFail({ email: dto.email.toLowerCase() });

    let device = await this.devices.findOne({
      where: {
        organizationId: user.organizationId,
        deviceFingerprint: dto.deviceFingerprint,
      },
    });

    if (!device) {
      device = this.devices.create({
        organizationId: user.organizationId,
        userId: user.id,
        deviceName: dto.deviceName,
        deviceFingerprint: dto.deviceFingerprint,
        status: DeviceStatus.ACTIVE,
      });
    }

    device.userId = user.id;
    device.deviceName = dto.deviceName;
    device.status = DeviceStatus.ACTIVE;
    device.lastSeenAt = new Date();
    device.tokenHash = this.hash(tokens.accessToken);
    await this.devices.save(device);

    const accessToken = await this.jwt.signAsync(
      {
        sub: user.id,
        email: user.email,
        orgId: user.organizationId,
        role: user.role,
        deviceId: device.id,
      },
      {
        secret: this.config.get('jwt.accessSecret'),
        expiresIn: this.config.get('jwt.accessTtl'),
      },
    );

    return {
      ...tokens,
      accessToken,
      device: { id: device.id, name: device.deviceName, status: device.status },
    };
  }

  private async issueTokens(user: User) {
    const payload = {
      sub: user.id,
      email: user.email,
      orgId: user.organizationId,
      role: user.role,
    };
    const accessToken = await this.jwt.signAsync(payload, {
      secret: this.config.get('jwt.accessSecret'),
      expiresIn: this.config.get('jwt.accessTtl'),
    });
    const refreshToken = randomBytes(48).toString('hex');
    const days = 7;
    await this.refreshTokens.save(
      this.refreshTokens.create({
        userId: user.id,
        tokenHash: this.hash(refreshToken),
        expiresAt: new Date(Date.now() + days * 24 * 60 * 60 * 1000),
      }),
    );
    return {
      accessToken,
      refreshToken,
      tokenType: 'Bearer',
      expiresIn: this.config.get('jwt.accessTtl'),
      user: {
        id: user.id,
        email: user.email,
        firstName: user.firstName,
        lastName: user.lastName,
        role: user.role,
        organizationId: user.organizationId,
      },
    };
  }

  private hash(value: string) {
    return createHash('sha256').update(value).digest('hex');
  }
}
