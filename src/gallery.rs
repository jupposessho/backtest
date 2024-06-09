use axum::{
    extract,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use backtest::chart::chart;
use charming::HtmlRenderer;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/:type/:name", get(render));

    axum::Server::bind(&"127.0.0.1:5555".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn render(
    extract::Path((r#type, name)): extract::Path<(String, String)>,
) -> impl IntoResponse {
    let renderer = HtmlRenderer::new(format!("{type} - {name}"), 1000, 800)
        .theme(charming::theme::Theme::Westeros);

    let chart = chart();
    Html(renderer.render(&chart).unwrap()).into_response()
}
