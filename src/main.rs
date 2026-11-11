use dioxus::prelude::*;

#[cfg(feature = "server")]
mod server;

const STYLE: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);

    #[cfg(feature = "server")]
    dioxus::serve(|| async move { server::router(App).await });
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shortened {
    pub id: i64,
    pub short_url: String,
}

/// 원본 URL을 받아 새 숫자 주소를 발급한다. 같은 URL을 다시 넣어도 새 번호를 발급한다.
#[post("/api/shorten")]
async fn shorten(url: String) -> ServerFnResult<Shortened> {
    server::shorten(url).await
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut result = use_signal(|| None::<Shortened>);
    let mut error = use_signal(|| None::<String>);
    let mut copied = use_signal(|| false);
    let mut busy = use_signal(|| false);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        busy.set(true);
        copied.set(false);
        match shorten(input().trim().to_string()).await {
            Ok(r) => {
                result.set(Some(r));
                error.set(None);
            }
            Err(ServerFnError::ServerError { message, .. }) => {
                result.set(None);
                error.set(Some(message));
            }
            Err(e) => {
                result.set(None);
                error.set(Some(format!("요청에 실패했습니다: {e}")));
            }
        }
        busy.set(false);
    };

    let copy = move |_| {
        if let Some(r) = result() {
            let js = format!("navigator.clipboard.writeText({:?})", r.short_url);
            document::eval(&js);
            copied.set(true);
        }
    };

    rsx! {
        document::Title { "숫자형 단축 URL" }
        document::Stylesheet { href: STYLE }
        main {
            h1 { "숫자형 단축 URL" }
            p { class: "hint", "긴 URL을 넣으면 숫자로 된 짧은 주소를 만들어 드립니다. 로그인은 필요 없습니다." }
            form { onsubmit: submit,
                input {
                    r#type: "url",
                    placeholder: "https://example.org/very/long/path?q=1#section",
                    required: true,
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                }
                button { r#type: "submit", disabled: busy(), if busy() { "생성 중…" } else { "단축하기" } }
            }
            if let Some(msg) = error() {
                p { class: "error", "{msg}" }
            }
            if let Some(r) = result() {
                div { class: "result",
                    a { href: "{r.short_url}", target: "_blank", "{r.short_url}" }
                    button { onclick: copy, if copied() { "복사됨" } else { "복사" } }
                }
            }
        }
    }
}
