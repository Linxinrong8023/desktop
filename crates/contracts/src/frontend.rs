use serde::Serialize;

/// Enumerates the HTTP methods supported by the generated frontend SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FrontendHttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// Describes one request field that the transport must interpolate into the URL path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendPathParam {
    pub rust_field_name: &'static str,
    pub wire_name: &'static str,
}

/// Describes one frontend-facing HTTP operation exported from `ora-contracts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendEndpoint {
    pub operation_name: &'static str,
    pub method: FrontendHttpMethod,
    pub path_template: &'static str,
    pub request_type: &'static str,
    pub response_type: &'static str,
    pub path_params: &'static [FrontendPathParam],
    pub has_json_body: bool,
}

pub const PROJECTS_PATH: &str = "/api/projects";
pub const PROJECT_PATH: &str = "/api/projects/{projectId}";
pub const PROJECT_WORK_CONTEXT_OPEN_PATH: &str = "/api/project-work-contexts/open";
pub const PROJECT_WORK_CONTEXT_RENEW_PATH: &str = "/api/project-work-contexts/renew";
pub const TASKS_PATH: &str = "/api/tasks";
pub const TASK_PATH: &str = "/api/tasks/{taskId}";
pub const SESSIONS_PATH: &str = "/api/sessions";
pub const SESSION_PATH: &str = "/api/sessions/{sessionId}";
pub const SKILLS_PATH: &str = "/api/skills";
pub const SKILL_PATH: &str = "/api/skills/{skillId}";
pub const AGENTS_PATH: &str = "/api/agents";
pub const AGENT_PATH: &str = "/api/agents/{agentId}";

const PROJECT_ID_PATH_PARAM: FrontendPathParam = FrontendPathParam {
    rust_field_name: "project_id",
    wire_name: "projectId",
};
const TASK_ID_PATH_PARAM: FrontendPathParam = FrontendPathParam {
    rust_field_name: "task_id",
    wire_name: "taskId",
};
const SESSION_ID_PATH_PARAM: FrontendPathParam = FrontendPathParam {
    rust_field_name: "session_id",
    wire_name: "sessionId",
};
const SKILL_ID_PATH_PARAM: FrontendPathParam = FrontendPathParam {
    rust_field_name: "skill_id",
    wire_name: "skillId",
};
const AGENT_ID_PATH_PARAM: FrontendPathParam = FrontendPathParam {
    rust_field_name: "agent_id",
    wire_name: "agentId",
};

const PROJECT_PATH_PARAMS: &[FrontendPathParam] = &[PROJECT_ID_PATH_PARAM];
const TASK_PATH_PARAMS: &[FrontendPathParam] = &[TASK_ID_PATH_PARAM];
const SESSION_PATH_PARAMS: &[FrontendPathParam] = &[SESSION_ID_PATH_PARAM];
const SKILL_PATH_PARAMS: &[FrontendPathParam] = &[SKILL_ID_PATH_PARAM];
const AGENT_PATH_PARAMS: &[FrontendPathParam] = &[AGENT_ID_PATH_PARAM];
const NO_PATH_PARAMS: &[FrontendPathParam] = &[];

const FRONTEND_ENDPOINTS: &[FrontendEndpoint] = &[
    FrontendEndpoint {
        operation_name: "createProject",
        method: FrontendHttpMethod::Post,
        path_template: PROJECTS_PATH,
        request_type: "CreateProjectRequest",
        response_type: "CreateProjectResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "getProject",
        method: FrontendHttpMethod::Get,
        path_template: PROJECT_PATH,
        request_type: "GetProjectRequest",
        response_type: "GetProjectResponse",
        path_params: PROJECT_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "listProjects",
        method: FrontendHttpMethod::Get,
        path_template: PROJECTS_PATH,
        request_type: "ListProjectsRequest",
        response_type: "ListProjectsResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "updateProject",
        method: FrontendHttpMethod::Put,
        path_template: PROJECT_PATH,
        request_type: "UpdateProjectRequest",
        response_type: "UpdateProjectResponse",
        path_params: PROJECT_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "deleteProject",
        method: FrontendHttpMethod::Delete,
        path_template: PROJECT_PATH,
        request_type: "DeleteProjectRequest",
        response_type: "DeleteProjectResponse",
        path_params: PROJECT_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "openProjectWorkContext",
        method: FrontendHttpMethod::Post,
        path_template: PROJECT_WORK_CONTEXT_OPEN_PATH,
        request_type: "OpenProjectWorkContextRequest",
        response_type: "OpenProjectWorkContextResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "renewProjectWorkContext",
        method: FrontendHttpMethod::Post,
        path_template: PROJECT_WORK_CONTEXT_RENEW_PATH,
        request_type: "RenewProjectWorkContextRequest",
        response_type: "RenewProjectWorkContextResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "createTask",
        method: FrontendHttpMethod::Post,
        path_template: TASKS_PATH,
        request_type: "CreateTaskRequest",
        response_type: "CreateTaskResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "getTask",
        method: FrontendHttpMethod::Get,
        path_template: TASK_PATH,
        request_type: "GetTaskRequest",
        response_type: "GetTaskResponse",
        path_params: TASK_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "listTasks",
        method: FrontendHttpMethod::Get,
        path_template: TASKS_PATH,
        request_type: "ListTasksRequest",
        response_type: "ListTasksResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "updateTask",
        method: FrontendHttpMethod::Put,
        path_template: TASK_PATH,
        request_type: "UpdateTaskRequest",
        response_type: "UpdateTaskResponse",
        path_params: TASK_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "deleteTask",
        method: FrontendHttpMethod::Delete,
        path_template: TASK_PATH,
        request_type: "DeleteTaskRequest",
        response_type: "DeleteTaskResponse",
        path_params: TASK_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "createSession",
        method: FrontendHttpMethod::Post,
        path_template: SESSIONS_PATH,
        request_type: "CreateSessionRequest",
        response_type: "CreateSessionResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "getSession",
        method: FrontendHttpMethod::Get,
        path_template: SESSION_PATH,
        request_type: "GetSessionRequest",
        response_type: "GetSessionResponse",
        path_params: SESSION_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "listSessions",
        method: FrontendHttpMethod::Get,
        path_template: SESSIONS_PATH,
        request_type: "ListSessionsRequest",
        response_type: "ListSessionsResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "updateSession",
        method: FrontendHttpMethod::Put,
        path_template: SESSION_PATH,
        request_type: "UpdateSessionRequest",
        response_type: "UpdateSessionResponse",
        path_params: SESSION_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "deleteSession",
        method: FrontendHttpMethod::Delete,
        path_template: SESSION_PATH,
        request_type: "DeleteSessionRequest",
        response_type: "DeleteSessionResponse",
        path_params: SESSION_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "createSkill",
        method: FrontendHttpMethod::Post,
        path_template: SKILLS_PATH,
        request_type: "CreateSkillRequest",
        response_type: "CreateSkillResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "getSkill",
        method: FrontendHttpMethod::Get,
        path_template: SKILL_PATH,
        request_type: "GetSkillRequest",
        response_type: "GetSkillResponse",
        path_params: SKILL_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "listSkills",
        method: FrontendHttpMethod::Get,
        path_template: SKILLS_PATH,
        request_type: "ListSkillsRequest",
        response_type: "ListSkillsResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "updateSkill",
        method: FrontendHttpMethod::Put,
        path_template: SKILL_PATH,
        request_type: "UpdateSkillRequest",
        response_type: "UpdateSkillResponse",
        path_params: SKILL_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "deleteSkill",
        method: FrontendHttpMethod::Delete,
        path_template: SKILL_PATH,
        request_type: "DeleteSkillRequest",
        response_type: "DeleteSkillResponse",
        path_params: SKILL_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "createAgent",
        method: FrontendHttpMethod::Post,
        path_template: AGENTS_PATH,
        request_type: "CreateAgentRequest",
        response_type: "CreateAgentResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "getAgent",
        method: FrontendHttpMethod::Get,
        path_template: AGENT_PATH,
        request_type: "GetAgentRequest",
        response_type: "GetAgentResponse",
        path_params: AGENT_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "listAgents",
        method: FrontendHttpMethod::Get,
        path_template: AGENTS_PATH,
        request_type: "ListAgentsRequest",
        response_type: "ListAgentsResponse",
        path_params: NO_PATH_PARAMS,
        has_json_body: false,
    },
    FrontendEndpoint {
        operation_name: "updateAgent",
        method: FrontendHttpMethod::Put,
        path_template: AGENT_PATH,
        request_type: "UpdateAgentRequest",
        response_type: "UpdateAgentResponse",
        path_params: AGENT_PATH_PARAMS,
        has_json_body: true,
    },
    FrontendEndpoint {
        operation_name: "deleteAgent",
        method: FrontendHttpMethod::Delete,
        path_template: AGENT_PATH,
        request_type: "DeleteAgentRequest",
        response_type: "DeleteAgentResponse",
        path_params: AGENT_PATH_PARAMS,
        has_json_body: false,
    },
];

/// Returns the Rust-owned endpoint metadata exported to the generated frontend SDK.
pub fn frontend_endpoints() -> &'static [FrontendEndpoint] {
    FRONTEND_ENDPOINTS
}

#[cfg(test)]
mod tests {
    use super::{
        AGENT_PATH, AGENTS_PATH, FrontendEndpoint, FrontendHttpMethod, FrontendPathParam,
        PROJECT_PATH, PROJECT_WORK_CONTEXT_OPEN_PATH, PROJECT_WORK_CONTEXT_RENEW_PATH,
        PROJECTS_PATH, SESSION_PATH, SESSIONS_PATH, SKILL_PATH, SKILLS_PATH, TASK_PATH, TASKS_PATH,
        frontend_endpoints,
    };
    use pretty_assertions::assert_eq;

    /// Verifies the exported endpoint manifest matches the current CRUD route surface.
    #[test]
    fn exports_frontend_endpoint_manifest() {
        assert_eq!(
            frontend_endpoints(),
            &[
                FrontendEndpoint {
                    operation_name: "createProject",
                    method: FrontendHttpMethod::Post,
                    path_template: PROJECTS_PATH,
                    request_type: "CreateProjectRequest",
                    response_type: "CreateProjectResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "getProject",
                    method: FrontendHttpMethod::Get,
                    path_template: PROJECT_PATH,
                    request_type: "GetProjectRequest",
                    response_type: "GetProjectResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "project_id",
                        wire_name: "projectId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "listProjects",
                    method: FrontendHttpMethod::Get,
                    path_template: PROJECTS_PATH,
                    request_type: "ListProjectsRequest",
                    response_type: "ListProjectsResponse",
                    path_params: &[],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "updateProject",
                    method: FrontendHttpMethod::Put,
                    path_template: PROJECT_PATH,
                    request_type: "UpdateProjectRequest",
                    response_type: "UpdateProjectResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "project_id",
                        wire_name: "projectId",
                    }],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "deleteProject",
                    method: FrontendHttpMethod::Delete,
                    path_template: PROJECT_PATH,
                    request_type: "DeleteProjectRequest",
                    response_type: "DeleteProjectResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "project_id",
                        wire_name: "projectId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "openProjectWorkContext",
                    method: FrontendHttpMethod::Post,
                    path_template: PROJECT_WORK_CONTEXT_OPEN_PATH,
                    request_type: "OpenProjectWorkContextRequest",
                    response_type: "OpenProjectWorkContextResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "renewProjectWorkContext",
                    method: FrontendHttpMethod::Post,
                    path_template: PROJECT_WORK_CONTEXT_RENEW_PATH,
                    request_type: "RenewProjectWorkContextRequest",
                    response_type: "RenewProjectWorkContextResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "createTask",
                    method: FrontendHttpMethod::Post,
                    path_template: TASKS_PATH,
                    request_type: "CreateTaskRequest",
                    response_type: "CreateTaskResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "getTask",
                    method: FrontendHttpMethod::Get,
                    path_template: TASK_PATH,
                    request_type: "GetTaskRequest",
                    response_type: "GetTaskResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "task_id",
                        wire_name: "taskId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "listTasks",
                    method: FrontendHttpMethod::Get,
                    path_template: TASKS_PATH,
                    request_type: "ListTasksRequest",
                    response_type: "ListTasksResponse",
                    path_params: &[],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "updateTask",
                    method: FrontendHttpMethod::Put,
                    path_template: TASK_PATH,
                    request_type: "UpdateTaskRequest",
                    response_type: "UpdateTaskResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "task_id",
                        wire_name: "taskId",
                    }],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "deleteTask",
                    method: FrontendHttpMethod::Delete,
                    path_template: TASK_PATH,
                    request_type: "DeleteTaskRequest",
                    response_type: "DeleteTaskResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "task_id",
                        wire_name: "taskId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "createSession",
                    method: FrontendHttpMethod::Post,
                    path_template: SESSIONS_PATH,
                    request_type: "CreateSessionRequest",
                    response_type: "CreateSessionResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "getSession",
                    method: FrontendHttpMethod::Get,
                    path_template: SESSION_PATH,
                    request_type: "GetSessionRequest",
                    response_type: "GetSessionResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "session_id",
                        wire_name: "sessionId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "listSessions",
                    method: FrontendHttpMethod::Get,
                    path_template: SESSIONS_PATH,
                    request_type: "ListSessionsRequest",
                    response_type: "ListSessionsResponse",
                    path_params: &[],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "updateSession",
                    method: FrontendHttpMethod::Put,
                    path_template: SESSION_PATH,
                    request_type: "UpdateSessionRequest",
                    response_type: "UpdateSessionResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "session_id",
                        wire_name: "sessionId",
                    }],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "deleteSession",
                    method: FrontendHttpMethod::Delete,
                    path_template: SESSION_PATH,
                    request_type: "DeleteSessionRequest",
                    response_type: "DeleteSessionResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "session_id",
                        wire_name: "sessionId",
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "createSkill",
                    method: FrontendHttpMethod::Post,
                    path_template: SKILLS_PATH,
                    request_type: "CreateSkillRequest",
                    response_type: "CreateSkillResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "getSkill",
                    method: FrontendHttpMethod::Get,
                    path_template: SKILL_PATH,
                    request_type: "GetSkillRequest",
                    response_type: "GetSkillResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "skill_id",
                        wire_name: "skillId"
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "listSkills",
                    method: FrontendHttpMethod::Get,
                    path_template: SKILLS_PATH,
                    request_type: "ListSkillsRequest",
                    response_type: "ListSkillsResponse",
                    path_params: &[],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "updateSkill",
                    method: FrontendHttpMethod::Put,
                    path_template: SKILL_PATH,
                    request_type: "UpdateSkillRequest",
                    response_type: "UpdateSkillResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "skill_id",
                        wire_name: "skillId"
                    }],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "deleteSkill",
                    method: FrontendHttpMethod::Delete,
                    path_template: SKILL_PATH,
                    request_type: "DeleteSkillRequest",
                    response_type: "DeleteSkillResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "skill_id",
                        wire_name: "skillId"
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "createAgent",
                    method: FrontendHttpMethod::Post,
                    path_template: AGENTS_PATH,
                    request_type: "CreateAgentRequest",
                    response_type: "CreateAgentResponse",
                    path_params: &[],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "getAgent",
                    method: FrontendHttpMethod::Get,
                    path_template: AGENT_PATH,
                    request_type: "GetAgentRequest",
                    response_type: "GetAgentResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "agent_id",
                        wire_name: "agentId"
                    }],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "listAgents",
                    method: FrontendHttpMethod::Get,
                    path_template: AGENTS_PATH,
                    request_type: "ListAgentsRequest",
                    response_type: "ListAgentsResponse",
                    path_params: &[],
                    has_json_body: false,
                },
                FrontendEndpoint {
                    operation_name: "updateAgent",
                    method: FrontendHttpMethod::Put,
                    path_template: AGENT_PATH,
                    request_type: "UpdateAgentRequest",
                    response_type: "UpdateAgentResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "agent_id",
                        wire_name: "agentId"
                    }],
                    has_json_body: true,
                },
                FrontendEndpoint {
                    operation_name: "deleteAgent",
                    method: FrontendHttpMethod::Delete,
                    path_template: AGENT_PATH,
                    request_type: "DeleteAgentRequest",
                    response_type: "DeleteAgentResponse",
                    path_params: &[FrontendPathParam {
                        rust_field_name: "agent_id",
                        wire_name: "agentId"
                    }],
                    has_json_body: false,
                },
            ]
        );
    }

    /// Verifies update operations describe the path/body split needed by the generated client.
    #[test]
    fn preserves_path_params_for_update_routes() {
        let update_task = frontend_endpoints()
            .iter()
            .find(|endpoint| endpoint.operation_name == "updateTask")
            .copied()
            .unwrap_or_else(|| panic!("missing updateTask endpoint"));

        assert_eq!(
            update_task,
            FrontendEndpoint {
                operation_name: "updateTask",
                method: FrontendHttpMethod::Put,
                path_template: TASK_PATH,
                request_type: "UpdateTaskRequest",
                response_type: "UpdateTaskResponse",
                path_params: &[FrontendPathParam {
                    rust_field_name: "task_id",
                    wire_name: "taskId",
                }],
                has_json_body: true,
            }
        );
    }

    /// Verifies the exported endpoint manifest omits backend-owned worktree operations.
    #[test]
    fn omits_worktree_endpoints_from_frontend_manifest() {
        assert_eq!(
            frontend_endpoints()
                .iter()
                .all(|endpoint| !endpoint.operation_name.contains("Worktree")),
            true
        );
    }

    /// Verifies catalogs publish separate collection and identifier resource routes.
    #[test]
    fn exports_skill_and_agent_crud_endpoints() {
        assert!(
            frontend_endpoints()
                .iter()
                .any(|endpoint| endpoint.operation_name == "updateSkill"
                    && endpoint.path_template == "/api/skills/{skillId}")
        );
        assert!(
            frontend_endpoints()
                .iter()
                .any(|endpoint| endpoint.operation_name == "updateAgent"
                    && endpoint.path_template == "/api/agents/{agentId}")
        );
    }
}
