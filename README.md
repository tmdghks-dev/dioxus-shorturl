# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: https://dioxus-shorturl.tmdghks.workers.dev
- 테스트 단축 URL: https://dioxus-shorturl.tmdghks.workers.dev/22
- 테스트 원본 URL: https://dioxuslabs.com/learn/0.7/?from=shorturl#getting-started

## 개요

Dioxus 0.7 풀스택(Rust) 하나의 크레이트로 화면과 서버를 모두 구현했습니다.

| 역할 | 위치 |
| --- | --- |
| 화면(입력·생성·결과·복사) | `src/main.rs`의 `App` 컴포넌트 (WASM으로 하이드레이션) |
| 생성 API | `src/main.rs`의 `#[post("/api/shorten")]` 서버 함수 → `src/server.rs::shorten` |
| `/숫자` 리디렉션·404·변경 거부 | `src/server.rs` (Dioxus 서버의 axum 라우터에 추가) |
| DB 스키마 | `migrations/schema.sql` (서버 시작 시 자동 적용) |
| Railway 설정 | `Dockerfile`, `railway.json` |
| Cloudflare 설정 | `cloudflare/wrangler.jsonc`, `cloudflare/src/index.js`, `cloudflare/build-assets.sh` |

로그인·회원가입은 없습니다. 누구나 `/`에서 URL을 넣고 단축 주소를 만들 수 있습니다.

## 구성과 요청 흐름

```
브라우저 ──HTTPS──▶ Cloudflare Worker (dioxus-shorturl.tmdghks.workers.dev)
                     ├─ /assets/*.wasm|js|css  → Workers Static Assets (Dioxus 웹 번들)
                     └─ 그 외 (/, /api/*, /123) → Railway 백엔드로 프록시 (redirect: manual)
                                                   └─ Postgres (Railway)
```

- Cloudflare가 공개 접점입니다. Dioxus 웹 번들(wasm·js·css)은 Cloudflare에서 직접 내려주고, 나머지 요청만 Railway로 넘깁니다.
- Worker는 `redirect: "manual"`로 프록시하므로 Railway의 `302 Location`이 그대로 브라우저에 전달됩니다. 공개 주소의 `/숫자`도 실제로 원본 URL로 이동합니다.
- `index.html`은 Cloudflare 자산에서 뺐기 때문에 `/`는 Railway가 SSR로 그리고, 브라우저가 Cloudflare의 wasm으로 하이드레이션합니다.

## 숫자 발급·중복 방지·불변성

- **발급 방식**: `links.id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY`. 번호는 Postgres 시퀀스가 `INSERT ... RETURNING id`로 발급합니다. 시퀀스의 `nextval`은 트랜잭션과 무관하게 원자적이라 동시 요청이 와도 같은 번호가 두 번 나오지 않고, PRIMARY KEY가 한 번 더 중복을 막습니다. `GENERATED ALWAYS`라 클라이언트가 번호를 지정할 수 없습니다.
- **같은 URL 재입력 규칙**: 같은 원본 URL을 다시 넣으면 **매번 새 번호를 발급**합니다. 기존 매핑은 건드리지 않습니다.
- **생성 후 불변**
  - 서버 코드에는 INSERT와 SELECT만 있고, 수정·삭제·만료 API가 없습니다.
  - `/{숫자}`로 오는 `POST`·`PUT`·`PATCH`·`DELETE`는 서버가 `405 Method Not Allowed`로 거부합니다.
  - DB 트리거 `links_immutable`이 `UPDATE`·`DELETE`, `links_no_truncate`가 `TRUNCATE`를 예외로 막습니다. 서버를 우회해도 매핑을 바꿀 수 없습니다.
  - 시퀀스는 되돌리지 않으므로 발급한 번호를 다른 주소에 재사용하지 않습니다.
- **입력 검증(서버)**: `url` 크레이트로 파싱해 `http`/`https` 스킴과 호스트가 있는 절대 URL만 받고, 2048자를 넘으면 거부합니다(400). DB에도 `CHECK (original_url ~* '^https?://')`가 있습니다.
- **리디렉션**: 저장한 원문을 그대로 `302 Found`의 `Location`으로 돌려주므로 경로·쿼리·프래그먼트(`#...`)가 유지됩니다.
- **404**: 숫자가 아니거나 없는 번호면 `404`와 안내 페이지를 보여줍니다.

## 로컬 실행

필요한 것: Rust, `wasm32-unknown-unknown` 타깃, Dioxus CLI 0.7, Docker(Postgres용).

```bash
rustup target add wasm32-unknown-unknown
cargo binstall dioxus-cli@0.7.10   # 또는 cargo install dioxus-cli --version 0.7.10

# DB 준비: Postgres만 띄우면 테이블·트리거는 서버가 시작할 때 만든다
docker run -d --name shorturl-pg -e POSTGRES_PASSWORD=dev -e POSTGRES_DB=shorturl -p 55432:5432 postgres:17-alpine

cp .env.example .env   # 값 수정
export DATABASE_URL=postgres://postgres:dev@localhost:55432/shorturl
export PUBLIC_BASE_URL=http://localhost:8080
dx serve --platform web
```

단위 테스트(URL 검증, 숫자 경로 파싱): `cargo test --features server`

## 환경변수

| 이름 | 어디 | 설명 |
| --- | --- | --- |
| `DATABASE_URL` | Railway 서비스 | Postgres 접속 문자열. Railway에서는 `${{Postgres.DATABASE_URL}}` 참조 |
| `PUBLIC_BASE_URL` | Railway 서비스 | 생성 결과에 보여줄 공개 주소(Cloudflare Worker 주소) |
| `PORT`, `IP` | Railway 서비스 | 바인드 주소. 기본 `0.0.0.0:8080` |
| `BACKEND_URL` | Cloudflare Worker `vars` | 프록시할 Railway 백엔드 주소 |

예시는 `.env.example`에 있습니다. 실제 DB 비밀번호·토큰은 저장소와 브라우저 코드에 넣지 않았고, Railway 변수로만 주입합니다. 브라우저 코드는 같은 출처의 `/api/shorten`만 호출합니다.

## 배포 방법

### Railway (백엔드 + DB)

```bash
railway init -n dioxus-shorturl
railway add -d postgres                     # Railway가 운영하는 Postgres
railway add -s dioxus-shorturl
railway variables -s dioxus-shorturl \
  --set 'DATABASE_URL=${{Postgres.DATABASE_URL}}' \
  --set PORT=8080 \
  --set PUBLIC_BASE_URL=https://dioxus-shorturl.tmdghks.workers.dev
railway domain -s dioxus-shorturl -p 8080   # → https://dioxus-shorturl-production.up.railway.app
railway up -s dioxus-shorturl
```

`railway.json`이 `Dockerfile` 빌드를 지정합니다. Dockerfile은 `dx bundle --platform web --release`로 서버 바이너리와 웹 자산을 함께 만들고, 런타임 이미지에서 `/app/server`를 실행합니다.

### Cloudflare (공개 접점 + 웹 번들)

```bash
cd cloudflare
./build-assets.sh        # 배포된 Railway 빌드의 wasm/js/css를 public/에 복사(index.html 제외) — 해시 이름이 백엔드 HTML과 일치
npx wrangler deploy      # → https://dioxus-shorturl.tmdghks.workers.dev
```

`wrangler.jsonc`의 `assets`는 `html_handling: "none"`, `not_found_handling: "none"`이라 정적 파일이 없는 경로는 모두 `src/index.js`(프록시)로 갑니다. `BACKEND_URL`에 Railway 주소를 넣습니다.

## 확인 시나리오와 결과

2026-11-11에 배포한 공개 주소(`https://dioxus-shorturl.tmdghks.workers.dev`)에서 확인한 결과입니다.

| 시나리오 | 재현 방법 | 결과 |
| --- | --- | --- |
| 로그인 없이 생성·복사 | 새 브라우저에서 서비스 URL 접속 → URL 입력 → "단축하기" → "복사" | `/22` 발급, 복사 버튼 동작 |
| 다른 브라우저에서 이동 | `curl -si https://dioxus-shorturl.tmdghks.workers.dev/22` | `302`, `location: https://dioxuslabs.com/learn/0.7/?from=shorturl#getting-started` (경로·쿼리·프래그먼트 유지) |
| 동시 생성 | 서로 다른 URL 20개를 동시에 `POST /api/shorten` | 20개 모두 다른 번호, 중복 0건 |
| 재배포 후 유지 | `railway redeploy` 후 `/1`, `/22` 재요청 | 같은 원본 URL로 `302` (데이터는 Railway Postgres 볼륨에 저장) |
| 잘못된 URL | `ftp://x`, `javascript:alert(1)` 로 생성 요청 | `400`, "http:// 또는 https:// 로 시작하는 URL만 사용할 수 있습니다." |
| 없는 숫자 | `GET /987654321` | `404` 안내 페이지 |
| 기존 연결 변경 시도 | `PUT`/`PATCH`/`DELETE`/`POST /1` | 모두 `405`, 매핑 그대로 |
| DB 직접 수정 시도 (로컬 Postgres) | `UPDATE links SET original_url=... WHERE id=1` | 트리거가 `links are immutable: UPDATE is not allowed`로 거부 |

API 직접 호출 예:

```bash
curl -s -X POST https://dioxus-shorturl.tmdghks.workers.dev/api/shorten \
  -H 'content-type: application/json' -d '{"url":"https://example.org/a?b=1#c"}'
# {"id":23,"short_url":"https://dioxus-shorturl.tmdghks.workers.dev/23"}
```
