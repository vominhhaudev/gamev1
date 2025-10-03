/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  // 1. Create Games Collection
  const gamesCollection = new Collection({
    "name": "games",
    "type": "base",
    "system": false,
    "schema": [
      {
        "name": "name",
        "type": "text",
        "required": true,
        "options": {
          "min": 1,
          "max": 100
        }
      },
      {
        "name": "max_players",
        "type": "number",
        "required": true,
        "options": {
          "min": 2,
          "max": 8
        }
      },
      {
        "name": "status",
        "type": "select",
        "required": true,
        "options": {
          "maxSelect": 1,
          "values": ["waiting", "playing", "finished"]
        }
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // 2. Create Players Collection
  const playersCollection = new Collection({
    "name": "players",
    "type": "base",
    "system": false,
    "schema": [
      {
        "name": "username",
        "type": "text",
        "required": true,
        "options": {
          "min": 3,
          "max": 50
        }
      },
      {
        "name": "email",
        "type": "email",
        "required": true,
        "options": {}
      },
      {
        "name": "score",
        "type": "number",
        "required": false,
        "options": {
          "min": 0
        }
      },
      {
        "name": "is_online",
        "type": "bool",
        "required": false
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // 3. Create Users Collection (Auth Collection)
  const usersCollection = new Collection({
    "name": "users",
    "type": "auth",
    "system": false,
    "schema": [
      {
        "name": "name",
        "type": "text",
        "required": false,
        "options": {
          "min": 1,
          "max": 100
        }
      },
      {
        "name": "avatar",
        "type": "file",
        "required": false,
        "options": {
          "maxSelect": 1,
          "maxSize": 5242880,
          "mimeTypes": ["image/jpeg", "image/png", "image/gif"],
          "thumbs": ["100x100"]
        }
      }
    ],
    "listRule": "id = @request.auth.id",
    "viewRule": "id = @request.auth.id",
    "createRule": "",
    "updateRule": "id = @request.auth.id",
    "deleteRule": "id = @request.auth.id"
  });

  // 4. Create Game Sessions Collection
  const sessionsCollection = new Collection({
    "name": "game_sessions",
    "type": "base",
    "system": false,
    "schema": [
      {
        "name": "game_id",
        "type": "relation",
        "required": true,
        "options": {
          "collectionId": gamesCollection.id,
          "cascadeDelete": true,
          "minSelect": 1,
          "maxSelect": 1
        }
      },
      {
        "name": "player_id",
        "type": "relation",
        "required": true,
        "options": {
          "collectionId": usersCollection.id,
          "cascadeDelete": true,
          "minSelect": 1,
          "maxSelect": 1
        }
      },
      {
        "name": "position",
        "type": "json",
        "required": true,
        "options": {
          "maxSize": 1000
        }
      },
      {
        "name": "session_score",
        "type": "number",
        "required": false,
        "options": {
          "min": 0
        }
      },
      {
        "name": "status",
        "type": "select",
        "required": true,
        "options": {
          "maxSelect": 1,
          "values": ["active", "finished"]
        }
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // Save all collections
  app.save(gamesCollection);
  app.save(playersCollection);
  app.save(usersCollection);
  app.save(sessionsCollection);
}, (app) => {
  // Rollback: Delete collections
  const games = app.findCollectionByNameOrId("games");
  const players = app.findCollectionByNameOrId("players");
  const users = app.findCollectionByNameOrId("users");
  const sessions = app.findCollectionByNameOrId("game_sessions");

  if (games) app.delete(games);
  if (players) app.delete(players);
  if (users) app.delete(users);
  if (sessions) app.delete(sessions);
});

