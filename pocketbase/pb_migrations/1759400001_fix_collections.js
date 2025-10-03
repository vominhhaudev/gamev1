/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  // Xóa collections cũ bị lỗi
  try {
    const oldGames = app.findCollectionByNameOrId("games");
    if (oldGames) app.delete(oldGames);
  } catch (e) {}

  try {
    const oldPlayers = app.findCollectionByNameOrId("players");
    if (oldPlayers) app.delete(oldPlayers);
  } catch (e) {}

  try {
    const oldSessions = app.findCollectionByNameOrId("game_sessions");
    if (oldSessions) app.delete(oldSessions);
  } catch (e) {}

  // Tạo lại games collection với cú pháp đúng
  const gamesCollection = new Collection({
    "name": "games",
    "type": "base",
    "system": false,
    "fields": [
      {
        "autogeneratePattern": "[a-z0-9]{15}",
        "hidden": false,
        "id": "text_id",
        "max": 15,
        "min": 15,
        "name": "id",
        "pattern": "^[a-z0-9]+$",
        "presentable": false,
        "primaryKey": true,
        "required": true,
        "system": true,
        "type": "text"
      },
      {
        "autogeneratePattern": "",
        "hidden": false,
        "id": "text_name",
        "max": 100,
        "min": 1,
        "name": "name",
        "pattern": "",
        "presentable": true,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "text"
      },
      {
        "hidden": false,
        "id": "number_max_players",
        "max": 8,
        "min": 2,
        "name": "max_players",
        "presentable": false,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "number"
      },
      {
        "hidden": false,
        "id": "select_status",
        "maxSelect": 1,
        "name": "status",
        "presentable": false,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "select",
        "values": ["waiting", "playing", "finished"]
      },
      {
        "hidden": false,
        "id": "autodate_created",
        "name": "created",
        "onCreate": true,
        "onUpdate": false,
        "presentable": false,
        "system": false,
        "type": "autodate"
      },
      {
        "hidden": false,
        "id": "autodate_updated",
        "name": "updated",
        "onCreate": true,
        "onUpdate": true,
        "presentable": false,
        "system": false,
        "type": "autodate"
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // Tạo lại players collection
  const playersCollection = new Collection({
    "name": "players",
    "type": "base",
    "system": false,
    "fields": [
      {
        "autogeneratePattern": "[a-z0-9]{15}",
        "hidden": false,
        "id": "text_id_players",
        "max": 15,
        "min": 15,
        "name": "id",
        "pattern": "^[a-z0-9]+$",
        "presentable": false,
        "primaryKey": true,
        "required": true,
        "system": true,
        "type": "text"
      },
      {
        "autogeneratePattern": "",
        "hidden": false,
        "id": "text_username",
        "max": 50,
        "min": 3,
        "name": "username",
        "pattern": "",
        "presentable": true,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "text"
      },
      {
        "hidden": false,
        "id": "email_email",
        "name": "email",
        "presentable": false,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "email",
        "exceptDomains": [],
        "onlyDomains": []
      },
      {
        "hidden": false,
        "id": "number_score",
        "max": null,
        "min": 0,
        "name": "score",
        "presentable": false,
        "primaryKey": false,
        "required": false,
        "system": false,
        "type": "number"
      },
      {
        "hidden": false,
        "id": "bool_is_online",
        "name": "is_online",
        "presentable": false,
        "primaryKey": false,
        "required": false,
        "system": false,
        "type": "bool"
      },
      {
        "hidden": false,
        "id": "autodate_created_players",
        "name": "created",
        "onCreate": true,
        "onUpdate": false,
        "presentable": false,
        "system": false,
        "type": "autodate"
      },
      {
        "hidden": false,
        "id": "autodate_updated_players",
        "name": "updated",
        "onCreate": true,
        "onUpdate": true,
        "presentable": false,
        "system": false,
        "type": "autodate"
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  app.save(gamesCollection);
  app.save(playersCollection);

  console.log("Fixed collections with correct schema");
}, (app) => {
  const games = app.findCollectionByNameOrId("games");
  const players = app.findCollectionByNameOrId("players");

  if (games) app.delete(games);
  if (players) app.delete(players);
});



