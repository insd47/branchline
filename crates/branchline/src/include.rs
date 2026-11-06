/// Includes the client that `branchline_build::Generator::compile` wrote to `OUT_DIR`.
///
/// The file is named after the root node in snake case, so `branchline::include!()` includes `api.rs` and
/// `branchline::include!("contest")` includes the client generated with `.root("Contest")`.
///
/// ```ignore
/// branchline::include!();
///
/// let api = Api::new(branchline::Http::new("https://api.example.com")?);
/// ```
#[macro_export]
macro_rules! include {
    () => {
        $crate::include!("api");
    };
    ($name:literal) => {
        ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/", $name, ".rs"));
    };
}
