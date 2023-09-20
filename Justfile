default:
    just --list

db:
    docker compose up --build -d db

adminer:
    docker compose up --build -d adminer

stop:
    docker compose down

full: db
    docker compose up --build