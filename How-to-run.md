# วิธีรันโปรเจกต์ (Next.js + Rust TrueMoney)

โฟลเดอร์โปรเจกต์: `C:\Users\patta\Desktop\reverz.in.TH`

## 1) ติดตั้งครั้งแรก
```powershell
cd C:\Users\patta\Desktop\reverz.in.TH\nextjs
npm install
```

cargo install wasm-pack
npm run build:wasm

## 2) สร้างไฟล์ `nextjs/.env.local`
```env
NEXT_PUBLIC_RUST_PROXY_BASE=/api/rust
RUST_API_BASE_URL=http://127.0.0.1:8081

# แนะนำให้ตั้งจริงในไฟล์นี้
TRUEMONEY_RECEIVER_PHONE=0931959423
JWT_SECRET=change-this-to-a-long-random-secret
REFRESH_TOKEN_SECRET=change-this-to-another-long-random-secret

TRUEMONEY_TIMEOUT_MS=10000
ACCESS_TOKEN_TTL_SECONDS=86400
REFRESH_TOKEN_TTL_SECONDS=2592000
TOPUP_RATE_LIMIT_PER_MINUTE=5
COOKIE_SECURE=false
ADMIN_USERNAME=root
ADMIN_PASSWORD=root
```

## 3) รันระบบ
เปิด 2 terminal

### Terminal A (Rust API)
```powershell
cd C:\Users\patta\Desktop\reverz.in.TH\nextjs
npm run dev:rust
```

หมายเหตุ:
- `dev:rust` จะ auto โหลดค่า env จาก `.env.local` และ `.env`
- ถ้าไม่มีค่า จะมี default สำหรับ dev ให้อัตโนมัติ

### Terminal B (Next.js)
```powershell
cd C:\Users\patta\Desktop\reverz.in.TH\nextjs
npm run dev
```

## 4) ทดสอบ
1. เปิด `http://localhost:3000/login`
2. เข้าระบบ
3. ไปที่ `Topup -> Topup method -> TrueMoney Wallet`
4. ทดสอบเติมด้วยลิงก์ `https://gift.truemoney.com/campaign/?v=...`

## 5) เช็กระบบเร็ว
```powershell
# Rust health
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:8081/health

# TypeScript
cd C:\Users\patta\Desktop\reverz.in.TH\nextjs
npx tsc --noEmit

# Rust tests
cd C:\Users\patta\Desktop\reverz.in.TH
cargo test --manifest-path nextjs/rust/src/Cargo.toml
```
