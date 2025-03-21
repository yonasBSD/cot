mod migrations;

use cot::auth::db::DatabaseUserApp;
use cot::cli::CliMetadata;
use cot::config::{DatabaseConfig, ProjectConfig};
use cot::db::migrations::SyncDynMigration;
use cot::db::{model, query, Auto, Model};
use cot::form::Form;
use cot::project::{WithApps, WithConfig};
use cot::request::{Request, RequestExt};
use cot::response::{Response, ResponseExt};
use cot::router::{Route, Router};
use cot::static_files::StaticFilesMiddleware;
use cot::{
    reverse_redirect, App, AppBuilder, Body, BoxedHandler, Project, ProjectContext, StatusCode,
};
use rinja::Template;

#[derive(Debug, Clone)]
#[model]
struct TodoItem {
    #[model(primary_key)]
    id: Auto<i32>,
    title: String,
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    request: &'a Request,
    todo_items: Vec<TodoItem>,
}

async fn index(request: Request) -> cot::Result<Response> {
    let todo_items = TodoItem::objects().all(request.db()).await?;
    let index_template = IndexTemplate {
        request: &request,
        todo_items,
    };
    let rendered = index_template.render()?;

    Ok(Response::new_html(StatusCode::OK, Body::fixed(rendered)))
}

#[derive(Debug, Form)]
struct TodoForm {
    #[form(opt(max_length = 100))]
    title: String,
}

async fn add_todo(mut request: Request) -> cot::Result<Response> {
    let todo_form = TodoForm::from_request(&mut request).await?.unwrap();

    {
        TodoItem {
            id: Auto::auto(),
            title: todo_form.title,
        }
        .save(request.db())
        .await?;
    }

    Ok(reverse_redirect!(request, "index")?)
}

async fn remove_todo(request: Request) -> cot::Result<Response> {
    let todo_id: i32 = request.path_params().parse()?;

    {
        query!(TodoItem, $id == todo_id)
            .delete(request.db())
            .await?;
    }

    Ok(reverse_redirect!(request, "index")?)
}

struct TodoApp;

impl App for TodoApp {
    fn name(&self) -> &'static str {
        "todo-app"
    }

    fn migrations(&self) -> Vec<Box<SyncDynMigration>> {
        cot::db::migrations::wrap_migrations(migrations::MIGRATIONS)
    }

    fn router(&self) -> Router {
        Router::with_urls([
            Route::with_handler_and_name("/", index, "index"),
            Route::with_handler_and_name("/todos/add", add_todo, "add-todo"),
            Route::with_handler_and_name("/todos/{todo_id}/remove", remove_todo, "remove-todo"),
        ])
    }
}

struct TodoProject;

impl Project for TodoProject {
    fn cli_metadata(&self) -> CliMetadata {
        cot::cli::metadata!()
    }

    fn config(&self, _config_name: &str) -> cot::Result<ProjectConfig> {
        Ok(ProjectConfig::builder()
            .debug(true)
            .database(DatabaseConfig::builder().url("sqlite::memory:").build())
            .build())
    }

    fn register_apps(&self, apps: &mut AppBuilder, _context: &ProjectContext<WithConfig>) {
        apps.register(DatabaseUserApp::new());
        apps.register_with_views(TodoApp, "");
    }

    fn middlewares(
        &self,
        handler: cot::project::RootHandlerBuilder,
        context: &ProjectContext<WithApps>,
    ) -> BoxedHandler {
        handler
            .middleware(StaticFilesMiddleware::from_context(context))
            .build()
    }
}

#[cot::main]
fn main() -> impl Project {
    TodoProject
}
