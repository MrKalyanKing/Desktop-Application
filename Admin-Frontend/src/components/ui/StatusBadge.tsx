import { cn } from '../../utils/cn';

const map: Record<string, string> = {
  completed: 'badge-green',
  synced: 'badge-green',
  connected: 'badge-green',
  scheduled: 'badge-amber',
  pending: 'badge-amber',
  in_progress: 'badge-blue',
  syncing: 'badge-blue',
  draft: 'badge',
  archived: 'badge',
  error: 'badge-rose',
  failed: 'badge-rose',
};

export function StatusBadge({ value }: { value: string }) {
  return <span className={cn('badge capitalize', map[value] ?? 'badge')}>{value.replace('_', ' ')}</span>;
}
