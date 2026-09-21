CREATE TABLE controller_binding (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    controller TEXT NOT NULL UNIQUE CHECK(length(controller)>0)
);
CREATE TABLE execution_controllers (
    execution TEXT PRIMARY KEY REFERENCES execution_identities(execution),
    controller TEXT NOT NULL REFERENCES controller_binding(controller)
);
CREATE TRIGGER bind_new_clone AFTER INSERT ON clone_executions BEGIN
    INSERT INTO execution_controllers SELECT NEW.execution,controller FROM controller_binding;
END;
