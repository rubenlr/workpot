CREATE TABLE repo_hidden_branches (
    repo_path TEXT NOT NULL,
    branch TEXT NOT NULL,
    PRIMARY KEY (repo_path, branch),
    FOREIGN KEY (repo_path) REFERENCES repos(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_hidden_branches_path ON repo_hidden_branches(repo_path);
