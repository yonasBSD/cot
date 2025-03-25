mod admin;
mod dbtest;
mod form;
mod main_fn;
mod model;
mod query;

use darling::Error;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

use crate::admin::impl_admin_model_for_struct;
use crate::dbtest::fn_to_dbtest;
use crate::form::impl_form_for_struct;
use crate::main_fn::{fn_to_cot_main, fn_to_cot_test};
use crate::model::impl_model_for_struct;
use crate::query::{Query, query_to_tokens};

#[proc_macro_derive(Form, attributes(form))]
pub fn derive_form(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let token_stream = impl_form_for_struct(&ast);
    token_stream.into()
}

#[proc_macro_derive(AdminModel)]
pub fn derive_admin_model(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let token_stream = impl_admin_model_for_struct(&ast);
    token_stream.into()
}

/// Implement the [`Model`] trait for a struct.
///
/// This macro will generate an implementation of the [`Model`] trait for the
/// given named struct. Note that all the fields of the struct **must**
/// implement the [`DatabaseField`] trait.
///
/// # Model types
///
/// The model type can be specified using the `model_type` parameter. The model
/// type can be one of the following:
///
/// * `application` (default): The model represents an actual table in a
///   normally running instance of the application.
/// ```
/// use cot::db::model;
///
/// #[model(model_type = "application")]
/// // This is equivalent to:
/// // #[model]
/// struct User {
///     #[model(primary_key)]
///     id: i32,
///     username: String,
/// }
/// ```
/// * `migration`: The model represents a table that is used for migrations. The
///   model name must be prefixed with an underscore. You shouldn't ever need to
///   use this type; the migration engine will generate the migration model
///   types for you.
///
///   Migration models have two major uses. The first is so that the migration
///   engine uses knows what was the state of model at the time the last
///   migration was generated. This allows the engine to automatically detect
///   the changes and generate the necessary migration code. The second use is
///   to allow custom code in the migrations: you might want the migration to
///   fill in some data, for instance. You can't use the actual model for this
///   because the model might have changed since the migration was generated.
///   You can, however, use the migration model, which will always represent
///   the state of the model at the time the migration runs.
/// ```
/// // In a migration file
/// use cot::db::model;
///
/// #[model(model_type = "migration")]
/// struct _User {
///     #[model(primary_key)]
///     id: i32,
///     username: String,
/// }
/// ```
/// * `internal`: The model represents a table that is used internally by Cot
///   (e.g. the `cot__migrations` table, storing which migrations have been
///   applied). They are ignored by the migration generator and should never be
///   used outside Cot code.
/// ```
/// use cot::db::model;
///
/// #[model(model_type = "internal")]
/// struct CotMigrations {
///     #[model(primary_key)]
///     id: i32,
///     app: String,
///     name: String,
/// }
/// ```
///
/// [`Model`]: trait.Model.html
/// [`DatabaseField`]: trait.DatabaseField.html
#[proc_macro_attribute]
pub fn model(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };
    let mut ast = parse_macro_input!(input as syn::DeriveInput);
    let token_stream = impl_model_for_struct(&attr_args, &mut ast);
    token_stream.into()
}

#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    let query_input = parse_macro_input!(input as Query);
    query_to_tokens(query_input).into()
}

#[proc_macro_attribute]
pub fn dbtest(_args: TokenStream, input: TokenStream) -> TokenStream {
    let fn_input = parse_macro_input!(input as ItemFn);
    fn_to_dbtest(fn_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// An attribute macro that defines an entry point to a Cot-powered app.
///
/// This macro is meant to wrap a function returning a structure implementing
/// [`CotProject`]. It should just initialize a [`CotProject`] and return it,
/// while the macro takes care of initializing an async runtime, creating a CLI
/// and running the app.
///
/// # Examples
///
/// ```no_run
/// use cot::project::RegisterAppsContext;
/// use cot::{App, AppBuilder, Project};
///
/// struct HelloApp;
///
/// impl App for HelloApp {
///     fn name(&self) -> &'static str {
///         env!("CARGO_PKG_NAME")
///     }
/// }
///
/// struct HelloProject;
/// impl Project for HelloProject {
///     fn register_apps(&self, apps: &mut AppBuilder, _context: &RegisterAppsContext) {
///         apps.register_with_views(HelloApp, "");
///     }
/// }
///
/// #[cot::main]
/// fn main() -> impl Project {
///     HelloProject
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let fn_input = parse_macro_input!(input as ItemFn);
    fn_to_cot_main(fn_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// An attribute macro that defines an `async` test function for a Cot-powered
/// app.
///
/// This is pretty much an equivalent to `#[tokio::test]` provided so that you
/// don't have to declare `tokio` as a dependency in your tests.
///
/// # Examples
///
/// ```no_run
/// use cot::test::TestDatabase;
///
/// #[cot::test]
/// async fn test_db() {
///     let db = TestDatabase::new_sqlite().await.unwrap();
///     // do something with the database
///     db.cleanup().await.unwrap();
/// }
/// ```
#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let fn_input = parse_macro_input!(input as ItemFn);
    fn_to_cot_test(&fn_input).into()
}

pub(crate) fn cot_ident() -> proc_macro2::TokenStream {
    let cot_crate = crate_name("cot").expect("cot is not present in `Cargo.toml`");
    match cot_crate {
        proc_macro_crate::FoundCrate::Itself => {
            quote! { ::cot }
        }
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { ::#ident }
        }
    }
}
