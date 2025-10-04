// Simple gRPC test script
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

async function testGrpcConnection() {
  try {
    console.log('Testing gRPC connection to localhost:50051...');

    // Load proto file
    const packageDefinition = protoLoader.loadSync(
      'proto/worker.proto',
      {
        keepCase: true,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true
      }
    );

    const proto = grpc.loadPackageDefinition(packageDefinition);

    // Create client
    const client = new proto.worker.v1.Worker(
      'localhost:50051',
      grpc.credentials.createInsecure()
    );

    console.log('gRPC client created successfully');

    // Test a simple call
    const joinRoomRequest = {
      room_id: 'test_room',
      player_id: 'test_player'
    };

    client.JoinRoom(joinRoomRequest, (error, response) => {
      if (error) {
        console.error('gRPC call failed:', error.message);
        console.error('Error details:', error);
      } else {
        console.log('gRPC call successful:', response);
      }
    });

  } catch (error) {
    console.error('Error setting up gRPC:', error);
  }
}

testGrpcConnection();
