# 🎮 GameV1 Gameplay Test Guide

## ✅ Current Status
- **Worker**: 🟢 Running on port 50051
- **Client**: 🟢 Running on port 5173

## 🚀 Testing Steps

### 1. Open Game Client
```
http://localhost:5173
```

### 2. Check Browser Console (F12)
Look for connection logs:
```
✅ Connected to gRPC server
✅ Joined room successfully
✅ Receiving game snapshots
```

### 3. Test Basic Gameplay

#### **Movement Controls**
- **WASD** or **Arrow Keys**: Move player
- **Mouse**: Look around (if 3D camera enabled)

#### **Game Elements to Test**
1. **Player Movement**: Character should move smoothly
2. **Collision Detection**: Should collide with obstacles
3. **Pickup Collection**: Walk into yellow cubes to collect
4. **Enemy Encounters**: Red enemies should chase player
5. **Power-ups**: Blue items give temporary boosts

### 4. Monitor Network Activity

#### **Check these in Browser Console:**
```javascript
// Connection status
console.log("WebSocket/gRPC connected:", connected);

// Game state updates
console.log("Tick:", gameState.tick);
console.log("Entities:", gameState.entities.length);

// Input processing
console.log("Input sent:", inputData);
```

#### **Check these in Worker Logs:**
```bash
# Monitor worker output
tail -f worker/worker_output.log

# Should see:
# - Player joined room
# - Input processing logs
# - Game state updates
```

### 5. Common Issues & Solutions

#### **❌ "Connection Failed"**
```bash
# Restart services
cargo run --bin worker
cd client && npm run dev
```

#### **❌ "No Game Updates"**
- Check browser console for WebSocket errors
- Verify worker is processing inputs

#### **❌ "Movement Not Working"**
- Check if player entity exists in game state
- Verify input is being sent to server

### 6. Debug Commands

#### **Check Active Connections**
```bash
# Windows PowerShell
netstat -ano | findstr :50051
netstat -ano | findstr :5173
```

#### **Test gRPC Directly**
```bash
# This would require gRPC client, but you can check if port is open
telnet 127.0.0.1 50051
```

### 7. Next Steps After Testing

✅ **If basic gameplay works:**
- Add more visual polish (3D models, animations)
- Implement multiplayer features
- Add sound effects and music

❌ **If issues found:**
- Debug connection problems
- Fix input processing
- Improve error handling

## 🎯 Success Criteria

✅ **Connection**: Client ↔ Worker communication working
✅ **Input**: Player movement responds to controls
✅ **Game State**: Game world updates and renders
✅ **Physics**: Collision detection and movement feel natural
✅ **Network**: Smooth multiplayer synchronization

## 📞 Need Help?

If you encounter issues:
1. Check browser console for errors
2. Monitor worker logs for exceptions
3. Verify both services are running
4. Test with a fresh browser session

**Current Status**: Both services running ✅
