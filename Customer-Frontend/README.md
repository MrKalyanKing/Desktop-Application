# Parakeet Studio — customer website

A separate candidate site from the admin dashboard. Premium espresso + champagne palette.

```bash
copy .env.example .env
npm install
npm run dev
```

Runs at `http://localhost:5175`.

- Landing, login, register
- Resume upload
- Practice interviews in the browser
- Personal performance notes (labeled as coaching, not hiring scores)

Live API: set `VITE_USE_MOCK=false` and use `/api/v1/portal/*`.
