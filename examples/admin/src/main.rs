use cot::__private::async_trait;
use cot::admin::AdminApp;
use cot::auth::db::{DatabaseUser, DatabaseUserApp};
use cot::cli::CliMetadata;
use cot::config::{DatabaseConfig, MiddlewareConfig, ProjectConfig, SessionMiddlewareConfig};
use cot::middleware::{AuthMiddleware, LiveReloadMiddleware, SessionMiddleware};
use cot::project::{MiddlewareContext, RegisterAppsContext};
use cot::response::{Response, ResponseExt};
use cot::router::{Route, Router, Urls};
use cot::static_files::StaticFilesMiddleware;
use cot::{App, AppBuilder, Body, BoxedHandler, Project, ProjectContext, StatusCode};
use rinja::Template;

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    urls: &'a Urls,
}

async fn index(urls: Urls) -> cot::Result<Response> {
    let index_template = IndexTemplate { urls: &urls };
    let rendered = index_template.render()?;

    Ok(Response::new_html(StatusCode::OK, Body::fixed(rendered)))
}

struct HelloApp;

#[async_trait]
impl App for HelloApp {
    fn name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    async fn init(&self, context: &mut ProjectContext) -> cot::Result<()> {
        // TODO use transaction
        let user = DatabaseUser::get_by_username(context.database(), "admin").await?;
        if user.is_none() {
            DatabaseUser::create_user(context.database(), "admin", "admin").await?;
        }

        Ok(())
    }

    fn router(&self) -> Router {
        Router::with_urls([Route::with_handler("/", index)])
    }
}

struct AdminProject;

impl Project for AdminProject {
    fn cli_metadata(&self) -> CliMetadata {
        cot::cli::metadata!()
    }

    fn config(&self, _config_name: &str) -> cot::Result<ProjectConfig> {
        Ok(ProjectConfig::builder()
            .debug(true)
            .database(
                DatabaseConfig::builder()
                    .url("sqlite://db.sqlite3?mode=rwc")
                    .build(),
            )
            .middlewares(
                MiddlewareConfig::builder()
                    .session(SessionMiddlewareConfig::builder().secure(false).build())
                    .build(),
            )
            .build())
    }

    fn register_apps(&self, apps: &mut AppBuilder, _context: &RegisterAppsContext) {
        apps.register(DatabaseUserApp::new());
        apps.register_with_views(AdminApp::new(), "/admin");
        apps.register_with_views(HelloApp, "");
    }

    fn middlewares(
        &self,
        handler: cot::project::RootHandlerBuilder,
        context: &MiddlewareContext,
    ) -> BoxedHandler {
        handler
            .middleware(StaticFilesMiddleware::from_context(context))
            .middleware(SessionMiddleware::from_context(context))
            .middleware(AuthMiddleware::new())
            .middleware(SessionMiddleware::new())
            .middleware(LiveReloadMiddleware::new())
            .build()
    }
}

#[cot::main]
fn main() -> impl Project {
    AdminProject
}
