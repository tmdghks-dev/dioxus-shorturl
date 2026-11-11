-- 숫자와 원본 URL의 매핑. 서버 시작 시 실행되며 여러 번 실행해도 안전하다.
CREATE TABLE IF NOT EXISTS links (
    -- 숫자 주소. IDENTITY 시퀀스로 발급하고 PRIMARY KEY로 중복을 막는다.
    -- GENERATED ALWAYS라 클라이언트가 번호를 직접 지정할 수 없다.
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    original_url TEXT        NOT NULL CHECK (original_url ~* '^https?://' AND length(original_url) <= 2048),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 생성 후 불변: UPDATE·DELETE는 DB 수준에서 거부한다.
CREATE OR REPLACE FUNCTION links_are_immutable() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'links are immutable: % is not allowed', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS links_immutable ON links;
CREATE TRIGGER links_immutable
    BEFORE UPDATE OR DELETE ON links
    FOR EACH ROW EXECUTE FUNCTION links_are_immutable();

DROP TRIGGER IF EXISTS links_no_truncate ON links;
CREATE TRIGGER links_no_truncate
    BEFORE TRUNCATE ON links
    FOR EACH STATEMENT EXECUTE FUNCTION links_are_immutable();
