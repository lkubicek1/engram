pub mod agents;
pub mod directive;
pub mod draft;
pub mod summary;
pub mod wrapper_cmd;
pub mod wrapper_sh;

pub use agents::AGENTS_TEMPLATE;
pub use directive::ROOT_DIRECTIVE_TEMPLATE;
pub use draft::DRAFT_TEMPLATE;
pub use summary::SUMMARY_TEMPLATE;
pub use wrapper_cmd::WRAPPER_CMD_TEMPLATE;
pub use wrapper_sh::WRAPPER_SH_TEMPLATE;
