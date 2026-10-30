import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import PDFDocument from 'pdfkit';
import { Repository } from 'typeorm';
import { ReportStatus } from '../../common/enums';
import { FileStorageService } from '../file-storage/file-storage.service';
import { InterviewSessionsService } from '../interview-sessions/interview-sessions.service';
import { InterviewAnalytics } from '../interview-analytics/interview-analytics.entity';
import { TranscriptSegment } from '../transcription/transcript-segment.entity';
import { InterviewReport } from './interview-report.entity';

@Injectable()
export class ReportsService {
  constructor(
    @InjectRepository(InterviewReport) private readonly reports: Repository<InterviewReport>,
    @InjectRepository(TranscriptSegment)
    private readonly segments: Repository<TranscriptSegment>,
    @InjectRepository(InterviewAnalytics)
    private readonly analytics: Repository<InterviewAnalytics>,
    private readonly sessions: InterviewSessionsService,
    private readonly files: FileStorageService,
  ) {}

  list(organizationId: string, sessionId?: string) {
    const qb = this.reports
      .createQueryBuilder('r')
      .innerJoin('r.session', 's')
      .where('s.organizationId = :organizationId', { organizationId })
      .orderBy('r.createdAt', 'DESC');
    if (sessionId) qb.andWhere('r.sessionId = :sessionId', { sessionId });
    return qb.getMany();
  }

  async get(organizationId: string, id: string) {
    const report = await this.reports.findOne({ where: { id }, relations: ['session'] });
    if (!report || report.session.organizationId !== organizationId) return null;
    return report;
  }

  async generate(organizationId: string, sessionId: string, userId: string, templateKey = 'session_summary') {
    const session = await this.sessions.get(organizationId, sessionId);
    const report = await this.reports.save(
      this.reports.create({
        sessionId,
        templateKey,
        status: ReportStatus.GENERATING,
        createdById: userId,
      }),
    );

    try {
      const segments = await this.segments.find({
        where: { sessionId },
        order: { sequence: 'ASC' },
      });
      const insights = await this.analytics.find({ where: { sessionId } });
      const pdf = await this.renderPdf(session.title, templateKey, segments, insights);
      const key = `org/${organizationId}/reports/${report.id}.pdf`;
      await this.files.put(key, pdf, 'application/pdf');
      report.storageKey = key;
      report.mimeType = 'application/pdf';
      report.status = ReportStatus.READY;
    } catch (error) {
      report.status = ReportStatus.FAILED;
      report.error = error instanceof Error ? error.message : 'Report generation failed';
    }

    return this.reports.save(report);
  }

  async download(organizationId: string, id: string) {
    const report = await this.get(organizationId, id);
    if (!report?.storageKey) return null;
    const buffer = await this.files.get(report.storageKey);
    return { buffer, report };
  }

  private renderPdf(
    title: string,
    templateKey: string,
    segments: TranscriptSegment[],
    insights: InterviewAnalytics[],
  ): Promise<Buffer> {
    return new Promise((resolve, reject) => {
      const doc = new PDFDocument({ margin: 48 });
      const chunks: Buffer[] = [];
      doc.on('data', (chunk) => chunks.push(chunk as Buffer));
      doc.on('end', () => resolve(Buffer.concat(chunks)));
      doc.on('error', reject);

      doc.fontSize(20).text('Parakeet AI Interview Report');
      doc.moveDown(0.3);
      doc.fontSize(12).fillColor('#444').text(title);
      doc.text(`Template: ${templateKey}`);
      doc.text(`Generated: ${new Date().toISOString()}`);
      doc.moveDown();
      doc.fillColor('#111').fontSize(14).text('Transcript');
      doc.moveDown(0.4);
      doc.fontSize(10);
      segments.forEach((segment) => {
        doc.fillColor('#0f766e').text(`${segment.speakerType} · seq ${segment.sequence}`);
        doc.fillColor('#111').text(segment.text);
        doc.moveDown(0.4);
      });
      doc.addPage();
      doc.fontSize(14).text('Insights (review required for AI-generated items)');
      doc.moveDown(0.5);
      insights.forEach((row) => {
        doc.fontSize(11).fillColor('#0f766e').text(`${row.kind} · ${row.origin} · review ${row.humanReviewStatus}`);
        doc.fontSize(9).fillColor('#111').text(JSON.stringify(row.payload, null, 2));
        doc.moveDown(0.6);
      });
      doc.end();
    });
  }
}
