//! Desktop bindings for workspace.

use super::{Binding, Permission};

pub(super) const BINDINGS: &[Binding] = &[
    Binding::Unary {
        operation: "listWorkspaces",
        handler: "commands::workspace::list_workspaces",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "getWorkspaceDiff",
        handler: "commands::workspace::get_workspace_diff",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "commitWorkspaceChanges",
        handler: "commands::workspace::commit_workspace_changes",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "pushWorkspaceBranch",
        handler: "commands::workspace::push_workspace_branch",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "getWorkspaceStatus",
        handler: "commands::workspace::get_workspace_status",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "stageWorkspaceChanges",
        handler: "commands::workspace::stage_workspace_changes",
        permission: Permission::MainWebview,
    },
    Binding::Unary {
        operation: "unstageWorkspaceChanges",
        handler: "commands::workspace::unstage_workspace_changes",
        permission: Permission::MainWebview,
    },
];
