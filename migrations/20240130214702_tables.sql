-- Add migration script here
CREATE TABLE IF NOT EXISTS keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  key_val VARCHAR(255) NOT NULL,
  claimed BOOLEAN DEFAULT FALSE NOT NULL,
  user_claim VARCHAR(255),
  claimed_at DATE,
  added_at DATE DEFAULT (datetime('now', 'localtime')),
  claim_round INTEGER,
  UNIQUE (key_val),
  FOREIGN KEY (user_claim) references users (id),
  FOREIGN KEY (claim_round) REFERENCES giveaway_rounds (round_id)
);

CREATE TABLE IF NOT EXISTS config (
  key VARCHAR(255) PRIMARY KEY NOT NULL,
  value VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS giveaway_rounds (
  round_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  status VARCHAR(255) NOT NULL -- e.g., 'active', 'completed'
);

CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  username VARCHAR(255) NOT NULL,
  UNIQUE (username)
);
