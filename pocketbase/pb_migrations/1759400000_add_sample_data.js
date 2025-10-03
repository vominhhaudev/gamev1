/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  // Create demo user for testing authentication
  const demoUserData = {
    "email": "admin@pocketbase.local",
    "password": "123456789",
    "passwordConfirm": "123456789",
    "name": "Demo User",
    "avatar": ""
  };

  try {
    // Check if user already exists
    const existingUsers = app.findRecordsByFilter("users", "email = 'admin@pocketbase.local'");
    if (existingUsers.length > 0) {
      console.log("Demo user already exists");
      return;
    }

    // Create new demo user
    const demoUser = new Record(app.findCollectionByNameOrId("users"), demoUserData);
    app.save(demoUser);
    console.log("Demo user created successfully");

  } catch (error) {
    console.error("Error creating demo user:", error);
  }

}, (app) => {
  // Rollback: Delete demo user
  try {
    const demoUsers = app.findRecordsByFilter("users", "email = 'admin@pocketbase.local'");
    demoUsers.forEach(user => {
      app.delete(user);
    });
    console.log("Demo user deleted");
  } catch (error) {
    console.error("Error deleting demo user:", error);
  }
});