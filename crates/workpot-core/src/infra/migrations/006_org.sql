ALTER TABLE repos ADD COLUMN notes TEXT NULL;
ALTER TABLE repos ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE repos ADD COLUMN pin_order INTEGER NULL;

CREATE TABLE repo_tags (
    repo_path TEXT NOT NULL,
    tag TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (repo_path, tag),
    FOREIGN KEY (repo_path) REFERENCES repos(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_tags_path ON repo_tags(repo_path);
CREATE INDEX idx_repo_tags_tag ON repo_tags(tag);
