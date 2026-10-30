import { Link } from 'react-router-dom';
import { ArrowRight, Mic, Sparkles, Trophy } from 'lucide-react';

const features = [
  {
    icon: Mic,
    title: 'Practice like the room is real',
    copy: 'Timed, structured interviews for mock, technical, HR, and general rounds — in the browser.',
  },
  {
    icon: Sparkles,
    title: 'See how you actually sound',
    copy: 'Track completeness, examples, and structure. Coaching notes are labeled, never treated as a verdict.',
  },
  {
    icon: Trophy,
    title: 'Keep every resume version',
    copy: 'Upload PDF or DOCX, attach it to a practice round, and watch your story get tighter over time.',
  },
];

export function LandingPage() {
  return (
    <div className="studio-bg overflow-hidden">
      <header className="mx-auto flex max-w-6xl items-center justify-between px-5 py-6">
        <div>
          <div className="text-[11px] uppercase tracking-[0.32em] text-[var(--gold)]">Parakeet</div>
          <div className="serif text-2xl">Studio</div>
        </div>
        <div className="flex gap-2">
          <Link to="/login" className="btn btn-ghost">Sign in</Link>
          <Link to="/register" className="btn btn-gold">Begin</Link>
        </div>
      </header>

      <section className="mx-auto grid max-w-6xl gap-10 px-5 pb-20 pt-8 lg:grid-cols-[1.2fr_0.8fr] lg:items-center">
        <div>
          <p className="chip mb-5">Candidate studio</p>
          <h1 className="max-w-xl text-5xl leading-[1.05] sm:text-6xl">
            Interview with the calm of someone who already practiced.
          </h1>
          <p className="mt-6 max-w-lg text-lg text-[var(--muted)]">
            A private web studio for resumes, mock interviews, and performance — designed to feel like a
            luxury atelier, not a generic job portal.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/register" className="btn btn-gold">
              Open your studio <ArrowRight size={16} />
            </Link>
            <Link to="/login" className="btn btn-ghost">I already have a room</Link>
          </div>
        </div>
        <div className="panel relative rounded-[32px] p-6">
          <div className="chip">Live practice</div>
          <h2 className="serif mt-4 text-2xl">Walk me through a production incident you owned.</h2>
          <p className="mt-4 text-sm leading-6 text-[var(--muted)]">
            Last quarter checkout p95 spiked. I led the investigation, isolated lock contention, shipped a
            fix, and wrote the postmortem...
          </p>
          <div className="mt-6 gold-line" />
          <div className="mt-4 flex justify-between text-xs uppercase tracking-[0.18em] text-[var(--gold)]">
            <span>Question 1 of 5</span>
            <span>Technical</span>
          </div>
        </div>
      </section>

      <div className="gold-line mx-auto max-w-6xl" />

      <section className="mx-auto grid max-w-6xl gap-4 px-5 py-16 md:grid-cols-3">
        {features.map((feature) => (
          <article key={feature.title} className="panel rounded-[28px] p-6">
            <feature.icon className="text-[var(--gold)]" size={22} />
            <h3 className="serif mt-4 text-xl">{feature.title}</h3>
            <p className="mt-3 text-sm leading-6 text-[var(--muted)]">{feature.copy}</p>
          </article>
        ))}
      </section>

      <section className="mx-auto max-w-6xl px-5 pb-20">
        <div className="panel rounded-[32px] p-8 sm:p-12">
          <h2 className="serif text-3xl sm:text-4xl">Three steps. No recruiter dashboard energy.</h2>
          <ol className="mt-8 grid gap-6 md:grid-cols-3">
            {['Upload the resume you actually send', 'Take a timed practice interview', 'Review measured notes, then try again'].map((step, i) => (
              <li key={step}>
                <div className="text-[var(--gold)]">0{i + 1}</div>
                <p className="mt-2 text-lg">{step}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>
    </div>
  );
}
