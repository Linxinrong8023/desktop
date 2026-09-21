CREATE TABLE controller_metadata (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    controller TEXT NOT NULL CHECK(length(controller)>0)
);
CREATE TABLE clone_operations (
    request TEXT PRIMARY KEY,
    operation TEXT NOT NULL UNIQUE,
    execution TEXT NOT NULL UNIQUE,
    node TEXT NOT NULL,
    input TEXT NOT NULL CHECK(json_valid(input)),
    result TEXT CHECK(result IS NULL OR json_valid(result))
);
CREATE TABLE clone_receipts (
    execution TEXT NOT NULL REFERENCES clone_operations(execution),
    sequence INTEGER NOT NULL,
    event TEXT NOT NULL CHECK(json_valid(event)),
    PRIMARY KEY(execution, sequence)
);
