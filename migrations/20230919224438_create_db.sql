CREATE TABLE pcbs (
    user_id                    BIGINT NOT NULL PRIMARY KEY, -- https://core.telegram.org/bots/api#user -> "[the user ID] has at most 52 significant bits"
    username                   TEXT   NOT NULL,
    latitude                   REAL   NOT NULL,
    longitude                  REAL   NOT NULL,
    additional_information     TEXT
)