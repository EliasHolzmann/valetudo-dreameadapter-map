CREATE TABLE pcbs (
    user_id                    BIGINT            NOT NULL PRIMARY KEY, -- https://core.telegram.org/bots/api#user -> "[the user ID] has at most 52 significant bits"
    username                   TEXT              NOT NULL,
    latitude                   DOUBLE PRECISION  NOT NULL,
    longitude                  DOUBLE PRECISION  NOT NULL,
    additional_information     TEXT
)