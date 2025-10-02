/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  const collection = app.findCollectionByNameOrId("pbc_2560181677")

  // update collection data
  unmarshal({
    "name": "eneegy"
  }, collection)

  return app.save(collection)
}, (app) => {
  const collection = app.findCollectionByNameOrId("pbc_2560181677")

  // update collection data
  unmarshal({
    "name": "gamev1"
  }, collection)

  return app.save(collection)
})
