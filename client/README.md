# gamev1 client

SvelteKit proof-of-concept ph?c v? Week 1 do?n networking.

## C�ch ch?y

```bash
npm install
npm run dev -- --open
```

Sau khi gateway ch?y t?i `ws://127.0.0.1:3000/ws`, m? route `/net-test` d? do round-trip.

## Thu m?c ch�nh

- `src/routes/net-test/+page.svelte`: UI do RTT v� hi?n th? m?u.
- `src/routes/+layout.svelte`: layout t?i gi?n.

> Luu �: d? �n chua commit node_modules; c?n Node 18+.
