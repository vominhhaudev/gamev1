<script>
    import { onMount, onDestroy } from 'svelte';
    import * as THREE from 'three';

    // Three.js setup - safe initialization
    let scene = null;
    let camera = null;
    let renderer = null;
    let animationId = null;

    // Game objects - simple declarations
    let playerMesh;
    let trackMeshes = [];
    let obstacleMeshes = [];
    let pickupMeshes = [];

    // Game state
    let isRunning = false;
    let isGameStarted = false;
    let playerPosition = { x: 0, y: 0, z: 0 };
    let cameraPosition = { x: 0, y: 8, z: 15 }; // Camera behind and above player
    let score = 0;
    let speed = 1;
    let gameTime = 0;

    // Lane system for endless runner
    const LANES = [-4, 0, 4]; // Wider lanes
    const LANE_WIDTH = 4;
    let currentLane = 1; // Middle lane (index 1)

    // Jump mechanics
    let isJumping = false;
    let jumpVelocity = 0;
    let JUMP_FORCE = 8;
    const GRAVITY = -25;

    // Slide mechanics
    let isSliding = false;
    const SLIDE_DURATION = 1000; // ms

    // Game settings
    const TRACK_SEGMENT_LENGTH = 25;
    const OBSTACLE_SPAWN_DISTANCE = 50;
    const MAX_TRACK_SEGMENTS = 8; // Reduced for better performance

    // Visual effects - simple declarations
    let particles = [];
    let trails = [];
    let screenShake = 0;
    let bloomEffect;

    // Animation states
    let playerScale = { x: 1, y: 1, z: 1 };
    let obstacleRotations = new Map();

    // Sound system
    let audioContext = null;
    let backgroundMusic = null;
    let soundBuffers = new Map();
    let isMusicPlaying = false;
    let masterVolume = 0.5;

    // Power-up system
    let activePowerUps = new Map();
    let powerUpEffects = new Map();

    // Performance optimization
    let frameCount = 0;
    let lastFpsUpdate = 0;
    let fps = 60;
    let particlePool = [];
    let maxParticles = 100;

    onMount(async () => {
        console.log('🚀 Starting Endless Runner 3D initialization...');
        initThreeJS();
        console.log('🔧 Three.js initialized, scene:', !!scene, 'camera:', !!camera, 'renderer:', !!renderer);
        setupLighting();
        setupFog();
        createPlayer();
        console.log('🎮 Player created, calling createInitialTrack...');
        createInitialTrack();
        createParticlePool();

        // Initialize audio after basic setup
        await initAudio();

        setupEventListeners();
        console.log('✅ Endless Runner 3D initialization complete');
    });

    onDestroy(() => {
        if (animationId) {
            cancelAnimationFrame(animationId);
        }
        if (renderer) {
            renderer.dispose();
        }
        // Remove event listeners that were actually added
        window.removeEventListener('keydown', handleKeyDown);
        window.removeEventListener('resize', onWindowResize);
    });

    function initThreeJS() {
        try {
            console.log('🔧 Initializing Three.js...');
            const container = document.getElementById('game3d-container');
            if (!container) {
                console.error('❌ Game 3D container not found');
                return;
            }
            console.log('✅ Container found');

            // Scene setup
            scene = new THREE.Scene();
            scene.background = new THREE.Color(0x87CEEB); // Sky blue

        // Camera setup (third-person follow)
        camera = new THREE.PerspectiveCamera(
            75, // Increased FOV to see more of the track
            window.innerWidth / window.innerHeight,
            0.1,
            2000
        );
        if (camera) {
            camera.position.set(cameraPosition.x, cameraPosition.y, cameraPosition.z);
            // Set initial camera look at to see the track and player
            camera.lookAt(0, 0, playerPosition.z - 5); // Look slightly ahead of player on the track
        }

        // Renderer setup với enhanced quality
        renderer = new THREE.WebGLRenderer({
            antialias: true,
            powerPreference: "high-performance",
            alpha: false
        });
        renderer.setSize(window.innerWidth * 0.8, window.innerHeight * 0.8);
        renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        renderer.shadowMap.enabled = true;
        renderer.shadowMap.type = THREE.PCFSoftShadowMap;
        renderer.shadowMap.autoUpdate = true;
        renderer.outputColorSpace = THREE.SRGBColorSpace;
        renderer.toneMapping = THREE.ACESFilmicToneMapping;
        renderer.toneMappingExposure = 1.2;
        container.appendChild(renderer.domElement);

        // Handle window resize
        window.addEventListener('resize', onWindowResize, false);

        console.log('✅ Three.js initialized successfully');
        } catch (error) {
            console.error('❌ Error initializing Three.js:', error);
        }
    }

    function onWindowResize() {
        if (!camera || !renderer) return;

        camera.aspect = window.innerWidth / window.innerHeight;
        camera.updateProjectionMatrix();
        renderer.setSize(window.innerWidth * 0.8, window.innerHeight * 0.8);
    }

    function setupLighting() {
        if (!scene) return;

        // Ambient light for overall illumination
        const ambientLight = new THREE.AmbientLight(0x606060, 0.6);
        scene.add(ambientLight);

        // Main directional light (sun) với enhanced shadows
        const directionalLight = new THREE.DirectionalLight(0xffffff, 1.5);
        directionalLight.position.set(100, 100, 50);
        directionalLight.castShadow = true;

        // Enhanced shadow configuration
        directionalLight.shadow.mapSize.width = 4096;
        directionalLight.shadow.mapSize.height = 4096;
        directionalLight.shadow.camera.near = 0.5;
        directionalLight.shadow.camera.far = 800;
        directionalLight.shadow.camera.left = -150;
        directionalLight.shadow.camera.right = 150;
        directionalLight.shadow.camera.top = 150;
        directionalLight.shadow.camera.bottom = -150;
        directionalLight.shadow.bias = -0.0001;

        scene.add(directionalLight);

        // Rim lighting for dramatic effect
        const rimLight = new THREE.DirectionalLight(0xFFE4B5, 0.4);
        rimLight.position.set(-100, 50, -50);
        scene.add(rimLight);

        // Point lights for atmosphere (disabled by default for performance)
        // const pointLight1 = new THREE.PointLight(0x4a9eff, 0.5, 100);
        // pointLight1.position.set(0, 10, 0);
        // scene.add(pointLight1);

        console.log('✅ Enhanced lighting setup complete');
    }

    function setupFog() {
        if (!scene) return;

        // Add fog for depth perception
        scene.fog = new THREE.Fog(0x87CEEB, 100, 500);
    }

    function createPlayer() {
        if (!scene) return;

        // Create player as a more detailed 3D character
        const playerGroup = new THREE.Group();

        // Body - enhanced material
        const bodyGeometry = new THREE.CapsuleGeometry(0.4, 1.2, 4, 8);
        const bodyMaterial = new THREE.MeshLambertMaterial({
            color: 0x4a9eff,
            emissive: 0x001122,
            emissiveIntensity: 0.1
        });
        const body = new THREE.Mesh(bodyGeometry, bodyMaterial);
        body.position.y = 0.6;
        body.castShadow = true;
        body.receiveShadow = true;
        playerGroup.add(body);

        // Head - enhanced material
        const headGeometry = new THREE.SphereGeometry(0.3, 8, 8);
        const headMaterial = new THREE.MeshLambertMaterial({
            color: 0xffdbac,
            emissive: 0x221100,
            emissiveIntensity: 0.05
        });
        const head = new THREE.Mesh(headGeometry, headMaterial);
        head.position.y = 1.8;
        head.castShadow = true;
        head.receiveShadow = true;
        playerGroup.add(head);

        // Arms - enhanced materials
        const armGeometry = new THREE.CapsuleGeometry(0.15, 0.8, 3, 6);
        const armMaterial = new THREE.MeshLambertMaterial({
            color: 0xffdbac,
            emissive: 0x221100,
            emissiveIntensity: 0.05
        });

        const leftArm = new THREE.Mesh(armGeometry, armMaterial);
        leftArm.position.set(-0.7, 1.0, 0);
        leftArm.castShadow = true;
        leftArm.receiveShadow = true;
        playerGroup.add(leftArm);

        const rightArm = new THREE.Mesh(armGeometry, armMaterial);
        rightArm.position.set(0.7, 1.0, 0);
        rightArm.castShadow = true;
        rightArm.receiveShadow = true;
        playerGroup.add(rightArm);

        // Legs - enhanced materials
        const legGeometry = new THREE.CapsuleGeometry(0.18, 0.9, 3, 6);
        const legMaterial = new THREE.MeshLambertMaterial({
            color: 0x333333,
            emissive: 0x111111,
            emissiveIntensity: 0.1
        });

        const leftLeg = new THREE.Mesh(legGeometry, legMaterial);
        leftLeg.position.set(-0.25, -0.7, 0);
        leftLeg.castShadow = true;
        leftLeg.receiveShadow = true;
        playerGroup.add(leftLeg);

        const rightLeg = new THREE.Mesh(legGeometry, legMaterial);
        rightLeg.position.set(0.25, -0.7, 0);
        rightLeg.castShadow = true;
        rightLeg.receiveShadow = true;
        playerGroup.add(rightLeg);

        // Set initial position at the start of the track
        playerGroup.position.set(LANES[currentLane], 0.1, 0); // Start at z=0
        playerMesh = playerGroup;
        scene.add(playerMesh);

        console.log('✅ Player created at position:', playerMesh.position);
        console.log('Player mesh children:', playerMesh.children.length);

        // Ensure player is visible
        playerMesh.visible = true;
        playerMesh.children.forEach(child => {
            child.visible = true;
        });
    }

    function createInitialTrack() {
        if (!scene) return;

        // Create initial track segments BEHIND the player (negative Z)
        for (let i = 0; i < 6; i++) {
            const trackSegment = createTrackSegment(-(i + 1) * TRACK_SEGMENT_LENGTH);
            trackMeshes.push(trackSegment);
            scene.add(trackSegment);
            console.log(`➕ Added track segment ${i} to scene at z: ${trackSegment.position.z}`);
            console.log(`📊 Track segment ${i} visible:`, trackSegment.visible, 'children:', trackSegment.children.length);
        }

        console.log(`✅ Initial track created with ${trackMeshes.length} segments`);
        console.log('Track meshes:', trackMeshes.map(mesh => mesh.position.z));
        console.log('First track segment position:', trackMeshes[0]?.position);
        console.log('First track segment children:', trackMeshes[0]?.children.length);
        console.log('Camera position:', camera?.position);
        console.log('Player position:', playerMesh?.position);
        console.log('Scene children count:', scene?.children.length);
        console.log('Track meshes in scene:', scene?.children.filter(child => trackMeshes.includes(child)).length);

        // Debug: Force render to check if track is visible
        console.log('🔄 Forcing render to check track visibility...');
        render();

        // Debug: Check if track segments are visible
        trackMeshes.forEach((mesh, index) => {
            console.log(`🔍 Track segment ${index}:`, {
                position: mesh.position,
                visible: mesh.visible,
                children: mesh.children.length,
                inScene: scene.children.includes(mesh)
            });

            // Check individual children visibility
            mesh.children.forEach((child, childIndex) => {
                console.log(`  ├── Child ${childIndex}:`, {
                    type: child.type,
                    visible: child.visible,
                    position: child.position,
                    material: child.material ? 'has material' : 'no material'
                });
            });
        });

        // Debug: Check if track is visible from camera
        if (trackMeshes[0] && camera) {
            const trackPos = trackMeshes[0].position;
            const cameraPos = camera.position;
            const distance = Math.sqrt(
                Math.pow(trackPos.x - cameraPos.x, 2) +
                Math.pow(trackPos.y - cameraPos.y, 2) +
                Math.pow(trackPos.z - cameraPos.z, 2)
            );
            console.log('Distance from camera to first track:', distance);
        }
    }

    function createTrackSegment(z) {
        const group = new THREE.Group();
        console.log(`🔧 Creating track segment at z: ${z}`);

        // Ground plane - wider and more visible
        const groundGeometry = new THREE.PlaneGeometry(50, TRACK_SEGMENT_LENGTH); // Even wider track
        const groundMaterial = new THREE.MeshLambertMaterial({
            color: 0x2d5a27,
            side: THREE.DoubleSide // Render both sides
        });
        const ground = new THREE.Mesh(groundGeometry, groundMaterial);
        ground.rotation.x = -Math.PI / 2;
        ground.position.set(0, -0.2, z + TRACK_SEGMENT_LENGTH / 2);
        ground.receiveShadow = true;
        group.add(ground);
        console.log(`✅ Ground added to track segment at z: ${z}, visible: ${ground.visible}`);

        // Lane markers (dashed lines) - brighter and more visible
        for (let i = 0; i < LANES.length; i++) {
            const laneMarkerGeometry = new THREE.BoxGeometry(0.8, 0.3, TRACK_SEGMENT_LENGTH);
            const laneMarkerMaterial = new THREE.MeshLambertMaterial({
                color: 0xffffff,
                emissive: 0xcccccc,
                emissiveIntensity: 0.3
            });
            const laneMarker = new THREE.Mesh(laneMarkerGeometry, laneMarkerMaterial);
            laneMarker.position.set(LANES[i], 0.15, z + TRACK_SEGMENT_LENGTH / 2);
            laneMarker.castShadow = true;
            group.add(laneMarker);
        }
        console.log(`✅ Lane markers added to track segment at z: ${z}`);

        // Side barriers - taller and more visible
        const barrierGeometry = new THREE.BoxGeometry(2, 4, TRACK_SEGMENT_LENGTH);
        const barrierMaterial = new THREE.MeshLambertMaterial({
            color: 0x8B4513,
            emissive: 0x442200,
            emissiveIntensity: 0.1
        });

        const leftBarrier = new THREE.Mesh(barrierGeometry, barrierMaterial);
        leftBarrier.position.set(-12, 2, z + TRACK_SEGMENT_LENGTH / 2);
        leftBarrier.castShadow = true;
        leftBarrier.receiveShadow = true;
        group.add(leftBarrier);

        const rightBarrier = new THREE.Mesh(barrierGeometry, barrierMaterial);
        rightBarrier.position.set(12, 2, z + TRACK_SEGMENT_LENGTH / 2);
        rightBarrier.castShadow = true;
        rightBarrier.receiveShadow = true;
        group.add(rightBarrier);

        console.log(`✅ Barriers added to track segment at z: ${z}`);
        console.log(`📊 Track segment at z: ${z} has ${group.children.length} children`);
        console.log(`📊 Group visible: ${group.visible}, position: ${group.position.z}`);

        // Ensure the entire group is visible
        group.visible = true;
        return group;
    }

    function setupEventListeners() {
        window.addEventListener('keydown', handleKeyDown);
    }

    function handleKeyDown(event) {
        if (!isGameStarted) {
            startGame();
            return;
        }

        switch (event.code) {
            case 'Space':
                event.preventDefault();
                if (!isJumping) {
                    jump();
                }
                break;
            case 'KeyA':
            case 'ArrowLeft':
                event.preventDefault();
                changeLane(-1);
                break;
            case 'KeyD':
            case 'ArrowRight':
                event.preventDefault();
                changeLane(1);
                break;
            case 'KeyS':
            case 'ArrowDown':
                event.preventDefault();
                if (!isSliding) {
                    slide();
                }
                break;
            case 'KeyR':
                event.preventDefault();
                resetGame();
                break;
        }
    }



    function slide() {
        if (isSliding || !playerMesh) return;

        isSliding = true;

        // Slide animation - squash and stretch
        if (playerMesh) {
            const originalScale = playerMesh.scale.clone();
            playerMesh.scale.y = 0.4;
            playerMesh.scale.z = 1.4;

            setTimeout(() => {
                if (playerMesh) {
                    playerMesh.scale.copy(originalScale);
                }
                isSliding = false;
            }, SLIDE_DURATION);
        }

        console.log('🛷 Player slid');
    }

    function changeLane(direction) {
        const newLane = currentLane + direction;

        if (newLane >= 0 && newLane < LANES.length && playerMesh) {
            currentLane = newLane;

            // Add smooth lane transition animation
            const startX = playerMesh.position.x;
            const targetX = LANES[currentLane];
            const duration = 200; // ms
            const startTime = Date.now();

            function animateLaneChange() {
                const elapsed = Date.now() - startTime;
                const progress = Math.min(elapsed / duration, 1);

                if (playerMesh) {
                    playerMesh.position.x = startX + (targetX - startX) * easeOutQuad(progress);
                }

                if (progress < 1) {
                    requestAnimationFrame(animateLaneChange);
                }
            }

            animateLaneChange();
            console.log(`🏃 Changed to lane ${currentLane + 1}`);
        }
    }

    function easeOutQuad(t) {
        return t * (2 - t);
    }

    function resetGame() {
        isGameStarted = false;
        isRunning = false;
        playerPosition = { x: 0, y: 0, z: 0 };
        cameraPosition = { x: 0, y: 8, z: 15 }; // Camera behind and above player
        score = 0;
        speed = 1;
        gameTime = 0;
        currentLane = 1;
        isJumping = false;
        isSliding = false;
        jumpVelocity = 0;

        // Reset player position
        if (playerMesh) {
            playerMesh.position.set(LANES[currentLane], 0.1, 0);
            playerMesh.scale.set(1, 1, 1);
        }

        // Reset camera
        if (camera) {
            camera.position.set(cameraPosition.x, cameraPosition.y, cameraPosition.z);
        }

        // Clear track segments
        trackMeshes.forEach(segment => {
            scene.remove(segment);
        });
        trackMeshes = [];

        // Clear obstacles
        obstacleMeshes.forEach(obstacle => {
            scene.remove(obstacle);
        });
        obstacleMeshes = [];

        // Clear pickups
        pickupMeshes.forEach(pickup => {
            scene.remove(pickup);
        });
        pickupMeshes = [];

        // Recreate initial track
        createInitialTrack();

        console.log('🔄 Game reset');
    }

    function gameLoop() {
        if (!isRunning) return;

        try {
            const deltaTime = 0.016; // ~60 FPS
            gameTime += deltaTime;

            updateGame(deltaTime);
            updatePhysics(deltaTime);
            updatePowerUps();
            updateCamera();
            updateFPS();
            render();

            animationId = requestAnimationFrame(gameLoop);
        } catch (error) {
            console.error('❌ Error in game loop:', error);
            isRunning = false;
        }
    }

    function updateGame(deltaTime) {
        if (!isGameStarted || !playerMesh) return;

        // Auto-run forward
        const runDistance = speed * 15 * deltaTime; // Base speed
        playerPosition.z += runDistance;

        // Update player horizontal position (smooth lane following)
        const targetX = LANES[currentLane];
        if (playerMesh.position.x !== targetX) {
            const diff = targetX - playerMesh.position.x;
            playerMesh.position.x += diff * 0.1; // Smooth interpolation
        }

        // Update score based on distance
        score = Math.floor(playerPosition.z);

        // Increase speed over time (difficulty ramp)
        speed = 1 + (gameTime * 0.01);

        // Generate new track segments (ensure we always have enough visible track)
        const segmentsNeeded = Math.ceil((camera.position.z - playerPosition.z + 50) / TRACK_SEGMENT_LENGTH) + 2;
        while (trackMeshes.length < segmentsNeeded) {
            generateNewTrack();
        }

        // Spawn obstacles periodically
        if (Math.random() < 0.01 && playerPosition.z > 20) { // 1% chance per frame after initial segment
            spawnObstacle();
        }

        // Spawn pickups occasionally
        if (Math.random() < 0.005 && playerPosition.z > 20) { // 0.5% chance per frame
            spawnPickup();
        }

        // Check collisions
        checkCollisions();
    }

    function updatePhysics(deltaTime) {
        if (!playerMesh) return;

        // Jump physics
        if (isJumping) {
            jumpVelocity += GRAVITY * deltaTime;
            playerMesh.position.y += jumpVelocity * deltaTime;

            // Check if landed
            if (playerMesh.position.y <= 0) {
                playerMesh.position.y = 0;
                isJumping = false;
                jumpVelocity = 0;

                // Create landing particles
                createLandingParticles();

                // Add screen shake for landing
                addScreenShake(0.2);
            }
        }

        // Update track positions to follow player (simple approach)
        trackMeshes.forEach((segment, index) => {
            // Position segments at fixed intervals behind player
            const segmentZ = playerPosition.z - (index + 1) * TRACK_SEGMENT_LENGTH;
            segment.position.z = segmentZ;
        });
    }

    function updateCamera() {
        if (!camera || !playerMesh) return;

        // Smooth camera following - camera stays behind player
        const targetX = playerMesh.position.x;
        const targetY = 8; // Camera height
        const targetZ = playerMesh.position.z + 15; // Camera behind player

        camera.position.x += (targetX - camera.position.x) * 0.08; // Faster following
        camera.position.y += (targetY - camera.position.y) * 0.08;
        camera.position.z += (targetZ - camera.position.z) * 0.08;

        // Look at the track ahead of the player
        const lookAheadDistance = 8;
        camera.lookAt(
            playerMesh.position.x,
            playerMesh.position.y + 1,
            playerMesh.position.z + lookAheadDistance
        );
    }

    function generateNewTrack() {
        // Add new track segment at the beginning (behind player)
        const lastSegmentZ = trackMeshes.length > 0 ? trackMeshes[0].position.z : playerPosition.z;
        const newZ = lastSegmentZ - TRACK_SEGMENT_LENGTH;
        const newSegment = createTrackSegment(newZ);
        trackMeshes.unshift(newSegment);
        scene.add(newSegment);

        // Remove old track segments if we have too many
        if (trackMeshes.length > MAX_TRACK_SEGMENTS) {
            const oldSegment = trackMeshes.pop();
            if (oldSegment) {
                scene.remove(oldSegment);
            }
        }
    }

    function spawnObstacle() {
        if (!scene) return;

        // Random lane and position ahead of player
        const laneIndex = Math.floor(Math.random() * LANES.length);
        const obstacleX = LANES[laneIndex];
        const obstacleZ = playerPosition.z + OBSTACLE_SPAWN_DISTANCE + (Math.random() * 50);

        // Random obstacle type
        const obstacleTypes = ['wall', 'spike', 'moving_platform'];
        const obstacleType = obstacleTypes[Math.floor(Math.random() * obstacleTypes.length)];

        // Create enhanced obstacle
        const obstacle = createObstacleMesh(obstacleType);
        obstacle.position.set(obstacleX, obstacleType === 'spike' ? 0 : 1, obstacleZ);
        obstacle.userData = { type: 'obstacle', lane: laneIndex, obstacleType };

        obstacleMeshes.push(obstacle);
        scene.add(obstacle);

        console.log(`🚧 Spawned ${obstacleType} obstacle at lane ${laneIndex + 1}, Z: ${obstacleZ}`);
        console.log('Obstacle position:', obstacle.position);
        console.log('Total obstacles:', obstacleMeshes.length);
    }

    function spawnPickup() {
        if (!scene) return;

        // Random lane and position ahead of player
        const laneIndex = Math.floor(Math.random() * LANES.length);
        const pickupX = LANES[laneIndex];
        const pickupZ = playerPosition.z + OBSTACLE_SPAWN_DISTANCE + (Math.random() * 30);

        // Create enhanced pickup
        const pickup = createPickupMesh();
        pickup.position.set(pickupX, 1.5, pickupZ);
        pickup.userData = { type: 'pickup', value: 100 };

        // Add floating animation
        const startTime = Date.now();
        pickup.userData.startTime = startTime;

        pickupMeshes.push(pickup);
        scene.add(pickup);

        console.log(`💰 Spawned pickup at lane ${laneIndex + 1}, Z: ${pickupZ}`);
    }



    function render() {
        if (renderer && scene && camera && isRunning) {
            try {
            // Update screen shake
            if (screenShake > 0) {
                const shakeX = (Math.random() - 0.5) * screenShake;
                const shakeY = (Math.random() - 0.5) * screenShake;
                camera.position.x += shakeX;
                camera.position.y += shakeY;
                screenShake *= 0.95; // Decay shake

                if (screenShake < 0.01) {
                    screenShake = 0;
                    // Reset camera position
                    camera.position.x = playerMesh ? playerMesh.position.x : 0;
                    camera.position.y = 8;
                    camera.position.z = playerMesh ? playerMesh.position.z + 15 : 15;
                }
            }

            // Update particles
            updateParticles();

            // Update trails
            updateTrails();

            // Animate pickups (floating effect)
            pickupMeshes.forEach(pickup => {
                if (pickup.userData.startTime) {
                    const time = (Date.now() - pickup.userData.startTime) / 1000;
                    pickup.position.y = 1.5 + Math.sin(time * 3) * 0.2;
                    pickup.rotation.y = time * 2;
                }
            });

            // Animate obstacles (rotation for some types)
            obstacleMeshes.forEach(obstacle => {
                if (obstacle.userData.obstacleType === 'spike') {
                    obstacle.rotation.y += 0.02;
                }
            });

                renderer.render(scene, camera);
            } catch (error) {
                console.error('❌ Error during render:', error);
            }
        }
    }

    function createJumpParticles() {
        if (!playerMesh || !scene) return;

        const particleCount = 15;
        for (let i = 0; i < particleCount; i++) {
            const angle = (i / particleCount) * Math.PI * 2;
            const radius = 2;
            const x = playerMesh.position.x + Math.cos(angle) * radius;
            const z = playerMesh.position.z + Math.sin(angle) * radius;
            const y = playerMesh.position.y + 0.5;

            particles.push(new THREE.Vector3(x, y, z));
        }
    }

    function updateParticles() {
        // Update existing particles using pooling
        particles = particles.filter(particle => {
            particle.y -= 0.1; // Fall down

            if (particle.y < -1) {
                // Return particle to pool
                if (particle.userData && particle.userData.mesh) {
                    returnParticleToPool(particle.userData.mesh);
                    delete particle.userData.mesh;
                }
                return false;
            }

            // Get pooled particle mesh
            if (!particle.userData || !particle.userData.mesh) {
                const pooledParticle = getPooledParticle();
                if (pooledParticle) {
                    particle.userData = { mesh: pooledParticle };
                }
            }

            // Update particle position
            if (particle.userData && particle.userData.mesh) {
                particle.userData.mesh.position.copy(particle);
            }

            return true;
        });
    }

    function updateTrails() {
        if (!playerMesh) return;

        // Add current position to trail
        trails.push(playerMesh.position.clone());

        // Limit trail length
        if (trails.length > 50) {
            trails.shift();
        }

        // Update trail meshes
        trails.forEach((position, index) => {
            if (!position.userData || !position.userData.mesh) {
                const geometry = new THREE.SphereGeometry(0.1, 6, 6);
                const material = new THREE.MeshBasicMaterial({
                    color: 0x4a9eff,
                    transparent: true,
                    opacity: 0.3
                });
                const mesh = new THREE.Mesh(geometry, material);
                scene.add(mesh);
                position.userData = { mesh };
            }

            if (position.userData && position.userData.mesh) {
                const mesh = position.userData.mesh;
                mesh.position.copy(position);

                // Fade out older trails
                const opacity = index / trails.length;
                if (mesh.material) {
                    mesh.material.opacity = opacity * 0.3;
                }
            }
        });

        // Remove very old trails
        if (trails.length > 30) {
            const oldTrail = trails[0];
            if (oldTrail && oldTrail.userData && oldTrail.userData.mesh) {
                scene.remove(oldTrail.userData.mesh);
            }
        }
    }

    function addScreenShake(intensity) {
        screenShake = Math.max(screenShake, intensity);
    }


    // Enhanced obstacle creation với animations
    function createObstacleMesh(obstacleType) {
        let geometry;
        let material;
        let scale = 1;

        switch (obstacleType) {
            case 'wall':
                geometry = new THREE.BoxGeometry(2, 3, 0.5);
                material = new THREE.MeshLambertMaterial({ color: 0x8B4513 });
                break;
            case 'spike':
                geometry = new THREE.ConeGeometry(0.8, 2, 8);
                material = new THREE.MeshLambertMaterial({ color: 0x444444 });
                scale = 0.8;
                break;
            case 'moving_platform':
                geometry = new THREE.BoxGeometry(4, 0.3, 3);
                material = new THREE.MeshLambertMaterial({ color: 0x654321 });
                break;
            default:
                geometry = new THREE.BoxGeometry(2, 2, 2);
                material = new THREE.MeshLambertMaterial({ color: 0xff4444 });
        }

        const mesh = new THREE.Mesh(geometry, material);
        mesh.scale.setScalar(scale);
        mesh.castShadow = true;
        mesh.receiveShadow = true;

        return mesh;
    }

    // Enhanced pickup creation
    function createPickupMesh() {
        const geometry = new THREE.OctahedronGeometry(0.6, 1);
        const material = new THREE.MeshLambertMaterial({
            color: 0xffd700,
            emissive: 0x442200,
            emissiveIntensity: 0.3
        });

        const mesh = new THREE.Mesh(geometry, material);
        mesh.castShadow = true;

        // Add glow effect
        const glowGeometry = new THREE.OctahedronGeometry(0.8, 1);
        const glowMaterial = new THREE.MeshBasicMaterial({
            color: 0xffd700,
            transparent: true,
            opacity: 0.6
        });
        const glow = new THREE.Mesh(glowGeometry, glowMaterial);
        mesh.add(glow);

        return mesh;
    }

    // Performance optimization functions
    function createParticlePool() {
        for (let i = 0; i < maxParticles; i++) {
            const geometry = new THREE.SphereGeometry(0.05, 4, 4);
            const material = new THREE.MeshBasicMaterial({
                color: 0x4a9eff,
                transparent: true,
                opacity: 0.8
            });
            const particle = new THREE.Mesh(geometry, material);
            particle.visible = false;
            particlePool.push(particle);
            scene.add(particle);
        }
    }

    function getPooledParticle() {
        const particle = particlePool.find(p => !p.visible);
        if (particle) {
            particle.visible = true;
            return particle;
        }
        return null;
    }

    function returnParticleToPool(particle) {
        particle.visible = false;
        particle.position.set(0, 0, 0);
    }

    function updateFPS() {
        frameCount++;
        const now = performance.now();

        if (now - lastFpsUpdate >= 1000) { // Update every second
            fps = Math.round((frameCount * 1000) / (now - lastFpsUpdate));
            frameCount = 0;
            lastFpsUpdate = now;

            // Adaptive quality based on FPS
            if (fps < 30 && maxParticles > 50) {
                maxParticles = 50; // Reduce particle count for better performance
            } else if (fps > 50 && maxParticles < 100) {
                maxParticles = 100; // Increase particle count when performance is good
            }
        }
    }

    // Power-up system functions
    function activatePowerUp(powerType, duration) {
        const startTime = Date.now();

        // Remove existing power-up of same type
        if (activePowerUps.has(powerType)) {
            deactivatePowerUp(powerType);
        }

        // Add new power-up
        activePowerUps.set(powerType, { type: powerType, duration, startTime });

        // Apply power-up effects
        applyPowerUpEffect(powerType);

        // Play power-up sound
        playSound('powerup', 0.6);

        console.log(`⚡ Activated ${powerType} for ${duration} seconds`);

        // Set timer to deactivate
        setTimeout(() => {
            deactivatePowerUp(powerType);
        }, duration * 1000);
    }

    function deactivatePowerUp(powerType) {
        if (!activePowerUps.has(powerType)) return;

        activePowerUps.delete(powerType);
        removePowerUpEffect(powerType);

        console.log(`⚡ Deactivated ${powerType}`);
    }

    function applyPowerUpEffect(powerType) {
        switch (powerType) {
            case 'speed_boost':
                // Increase speed multiplier
                speed *= 1.5;
                // Add visual effect to player body parts
                if (playerMesh) {
                    playerMesh.children.forEach(child => {
                        if (child.material && child.material.emissive) {
                            child.material.emissive = new THREE.Color(0x4a9eff);
                            child.material.emissiveIntensity = 0.3;
                        }
                    });
                }
                break;

            case 'jump_boost':
                // Increase jump height
                JUMP_FORCE *= 1.3;
                // Add visual effect
                if (playerMesh) {
                    playerMesh.children.forEach(child => {
                        if (child.material && child.material.emissive) {
                            child.material.emissive = new THREE.Color(0x00ff00);
                            child.material.emissiveIntensity = 0.2;
                        }
                    });
                }
                break;

            case 'invincibility':
                // Make player invincible (handled in collision detection)
                if (playerMesh) {
                    playerMesh.children.forEach(child => {
                        if (child.material && child.material.emissive) {
                            child.material.emissive = new THREE.Color(0xffaa00);
                            child.material.emissiveIntensity = 0.4;
                        }
                    });
                }
                break;
        }
    }

    function removePowerUpEffect(powerType) {
        // Reset player appearance
        if (playerMesh) {
            playerMesh.children.forEach(child => {
                if (child.material && child.material.emissive) {
                    child.material.emissive = new THREE.Color(0x000000);
                    child.material.emissiveIntensity = 0;
                }
            });
        }

        // Reset game mechanics
        switch (powerType) {
            case 'speed_boost':
                speed /= 1.5;
                break;

            case 'jump_boost':
                JUMP_FORCE /= 1.3;
                break;
        }
    }

    function updatePowerUps() {
        // Update power-up timers and effects
        activePowerUps.forEach((powerUp, type) => {
            const elapsed = (Date.now() - powerUp.startTime) / 1000;
            const remaining = Math.max(0, powerUp.duration - elapsed);

            // Update UI if needed
            // This could show remaining time for active power-ups
        });
    }

    // Sound system functions
    async function initAudio() {
        try {
            if (typeof window !== 'undefined') {
                audioContext = new (window.AudioContext || window.webkitAudioContext)();
            }

            // Load sound effects only if audioContext is available
            if (audioContext) {
                await loadSoundEffects();
                console.log('✅ Audio system initialized');
            } else {
                console.warn('❌ AudioContext not available');
            }
        } catch (error) {
            console.warn('❌ Failed to initialize audio:', error);
        }
    }

    async function loadSoundEffects() {
        // Create procedural sound effects since we don't have audio files
        // In a real game, you would load actual audio files here

        // Jump sound - short ascending tone
        const jumpBuffer = createProceduralSound(0.3, (time) => {
            return Math.sin(2 * Math.PI * 440 * time) * Math.exp(-time * 3) * 0.3;
        });
        if (jumpBuffer) soundBuffers.set('jump', jumpBuffer);

        // Landing sound - short descending tone
        const landingBuffer = createProceduralSound(0.2, (time) => {
            return Math.sin(2 * Math.PI * 220 * time) * Math.exp(-time * 5) * 0.4;
        });
        if (landingBuffer) soundBuffers.set('landing', landingBuffer);

        // Pickup sound - pleasant ascending arpeggio
        const pickupBuffer = createProceduralSound(0.4, (time) => {
            const note1 = Math.sin(2 * Math.PI * 523 * time) * Math.exp(-time * 2); // C5
            const note2 = Math.sin(2 * Math.PI * 659 * time * 1.1) * Math.exp(-time * 2) * 0.7; // E5
            const note3 = Math.sin(2 * Math.PI * 784 * time * 1.2) * Math.exp(-time * 2) * 0.5; // G5
            return (note1 + note2 + note3) * 0.2;
        });
        if (pickupBuffer) soundBuffers.set('pickup', pickupBuffer);

        // Collision sound - harsh noise
        const collisionBuffer = createProceduralSound(0.5, (time) => {
            return (Math.random() - 0.5) * Math.exp(-time * 2) * 0.6;
        });
        if (collisionBuffer) soundBuffers.set('collision', collisionBuffer);

        // Power-up sound - magical ascending chord
        const powerupBuffer = createProceduralSound(0.8, (time) => {
            const note1 = Math.sin(2 * Math.PI * 523 * time) * Math.exp(-time * 1); // C5
            const note2 = Math.sin(2 * Math.PI * 659 * time) * Math.exp(-time * 1) * 0.8; // E5
            const note3 = Math.sin(2 * Math.PI * 784 * time) * Math.exp(-time * 1) * 0.6; // G5
            return (note1 + note2 + note3) * 0.3;
        });
        if (powerupBuffer) soundBuffers.set('powerup', powerupBuffer);

        console.log('✅ Sound effects loaded');
    }

    function createProceduralSound(duration, waveFunction) {
        if (!audioContext) return null;

        const sampleRate = audioContext.sampleRate;
        const length = sampleRate * duration;
        const buffer = audioContext.createBuffer(1, length, sampleRate);
        const data = buffer.getChannelData(0);

        for (let i = 0; i < length; i++) {
            const time = i / sampleRate;
            data[i] = waveFunction(time);
        }

        return buffer;
    }

    function playSound(soundName, volume = 0.5) {
        if (!audioContext || !soundBuffers.has(soundName)) return;

        try {
            const buffer = soundBuffers.get(soundName);
            if (!buffer || !audioContext) return;

            const source = audioContext.createBufferSource();
            const gainNode = audioContext.createGain();

            source.buffer = buffer;
            source.connect(gainNode);
            gainNode.connect(audioContext.destination);

            gainNode.gain.setValueAtTime(volume * masterVolume, audioContext.currentTime);

            source.start();
        } catch (error) {
            console.warn('❌ Failed to play sound:', soundName, error);
        }
    }

    async function startBackgroundMusic() {
        if (!audioContext || isMusicPlaying) return;

        try {
            // Create a simple background melody loop
            const musicBuffer = createBackgroundMusic();
            if (!musicBuffer) return;

            backgroundMusic = audioContext.createBufferSource();
            const gainNode = audioContext.createGain();

            backgroundMusic.buffer = musicBuffer;
            backgroundMusic.loop = true;
            backgroundMusic.connect(gainNode);
            gainNode.connect(audioContext.destination);

            gainNode.gain.setValueAtTime(masterVolume * 0.3, audioContext.currentTime);

            backgroundMusic.start();
            isMusicPlaying = true;

            console.log('🎵 Background music started');
        } catch (error) {
            console.warn('❌ Failed to start background music:', error);
        }
    }

    function stopBackgroundMusic() {
        if (backgroundMusic && isMusicPlaying) {
            backgroundMusic.stop();
            isMusicPlaying = false;
            console.log('🎵 Background music stopped');
        }
    }

    function createBackgroundMusic() {
        if (!audioContext) return null;

        const duration = 4; // 4 second loop
        const sampleRate = audioContext.sampleRate;
        const length = sampleRate * duration;
        const buffer = audioContext.createBuffer(1, length, sampleRate);
        const data = buffer.getChannelData(0);

        // Simple melody pattern
        const notes = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 493.88, 523.25]; // C4 to C6 scale

        for (let i = 0; i < length; i++) {
            const time = i / sampleRate;
            const beat = Math.floor(time * 2) % 8; // 2 beats per second, 8 note pattern
            const frequency = notes[beat] || notes[0];

            // Create a simple square wave with some harmonics
            const wave = Math.sin(2 * Math.PI * frequency * time) > 0 ? 1 : -1;
            const harmonic = Math.sin(2 * Math.PI * frequency * 2 * time) * 0.3;

            data[i] = (wave + harmonic) * 0.1 * Math.exp(-time * 0.1); // Fade out over time
        }

        return buffer;
    }

    function toggleMusic() {
        if (isMusicPlaying) {
            stopBackgroundMusic();
        } else {
            startBackgroundMusic();
        }
    }

    // Main game start function with audio initialization
    function startGame() {
        console.log('🎮 Starting game...');
        isGameStarted = true;
        isRunning = true;
        gameTime = 0;

        // Initialize audio on first user interaction
        if (!audioContext) {
            initAudio();
        }

        // Start background music
        setTimeout(() => {
            startBackgroundMusic();
        }, 500);

        console.log('🎮 Endless Runner 3D started!');

        // Ensure we start the game loop
        if (!animationId && renderer && scene && camera) {
            console.log('🚀 Starting game loop...');
            gameLoop();
        } else {
            console.warn('⚠️ Cannot start game loop - missing required components');
        }
    }

    // Update jump function to play sound
    function jump() {
        if (isJumping || !playerMesh) return;

        isJumping = true;
        jumpVelocity = JUMP_FORCE;

        // Add jump animation effect với enhanced visuals
        if (playerMesh) {
            playerMesh.scale.y = 1.1; // Slight squash effect
            setTimeout(() => {
                if (playerMesh) playerMesh.scale.y = 1.0;
            }, 150);
        }

        // Create jump particles
        createJumpParticles();

        // Add screen shake
        addScreenShake(0.3);

        // Play jump sound
        playSound('jump', 0.6);

        console.log('🏃 Player jumped');
    }

    // Update landing to play sound
    function createLandingParticles() {
        if (!playerMesh) return;

        const particleCount = 8;
        for (let i = 0; i < particleCount; i++) {
            const angle = (i / particleCount) * Math.PI * 2;
            const radius = 1.5;
            const x = playerMesh.position.x + Math.cos(angle) * radius;
            const z = playerMesh.position.z + Math.sin(angle) * radius;
            const y = 0.1;

            particles.push(new THREE.Vector3(x, y, z));
        }

        // Play landing sound
        playSound('landing', 0.4);
    }

    // Update collision check to play sound
    function gameOver() {
        isRunning = false;

        // Play collision sound
        playSound('collision', 0.8);

        // Stop background music
        stopBackgroundMusic();

        console.log(`💀 Game Over! Final Score: ${score}`);

        // Show game over screen or restart option
        setTimeout(() => {
            if (confirm(`Game Over! Score: ${score}. Play again?`)) {
                resetGame();
                startGame();
            }
        }, 100);
    }

    // Update pickup collection to play sound
    function checkCollisions() {
        if (!playerMesh) return;

        const playerBox = new THREE.Box3().setFromObject(playerMesh);

        // Check obstacle collisions
        obstacleMeshes = obstacleMeshes.filter(obstacle => {
            const obstacleBox = new THREE.Box3().setFromObject(obstacle);
            if (playerBox.intersectsBox(obstacleBox)) {
                // Collision detected
                if (!isSliding && !isJumping) {
                    // Check for invincibility power-up
                    const hasInvincibility = activePowerUps.has('invincibility');

                    if (!hasInvincibility) {
                        playSound('collision', 0.7); // Play collision sound
                        gameOver();
                        return false;
                    } else {
                        // Invincible - destroy obstacle instead
                        scene.remove(obstacle);
                        playSound('collision', 0.3); // Quieter collision sound for invincibility
                        addScreenShake(0.1); // Small shake for feedback
                        console.log('🛡️ Invincibility protected player from obstacle!');
                        return false;
                    }
                }
            }

            // Remove obstacles that are behind player
            if (obstacle.position.z < playerPosition.z - 20) {
                scene.remove(obstacle);
                return false;
            }

            return true;
        });


        // Check power-up collisions (from backend entities)
        // Note: This would be handled by the backend entity system
        // For now, we'll simulate power-up spawning in the frontend
        if (Math.random() < 0.002 && playerPosition.z > 50) { // 0.2% chance per frame
            const powerTypes = ['speed_boost', 'jump_boost', 'invincibility'];
            const randomPower = powerTypes[Math.floor(Math.random() * powerTypes.length)];

            // Simulate power-up activation (in real implementation, this would come from backend)
            setTimeout(() => {
                activatePowerUp(randomPower, 8); // 8 seconds duration
            }, 100);
        }
    }
</script>

<div class="endless-runner-container">
    {#if !isGameStarted}
        <div class="start-screen">
            <div class="start-content">
                <h1>🏃 Endless Runner 3D</h1>
                <p>Press any key to start!</p>
                <div class="controls-preview">
                    <div class="control-item">
                        <span class="key">SPACE</span> Jump
                    </div>
                    <div class="control-item">
                        <span class="key">A/D</span> Change Lane
                    </div>
                    <div class="control-item">
                        <span class="key">S</span> Slide
                    </div>
                </div>
            </div>
        </div>
    {/if}

    <div class="game-header">
        <div class="score">Score: {score.toLocaleString()}</div>
        <div class="speed">Speed: {speed.toFixed(1)}x</div>
        <div class="distance">Distance: {Math.floor(playerPosition.z)}m</div>
        <div class="fps {fps >= 50 ? 'fps-high' : fps >= 30 ? 'fps-medium' : 'fps-low'}">FPS: {fps}</div>
        <button class="audio-btn" on:click={toggleMusic} title="Toggle Background Music">
            {isMusicPlaying ? '🔊' : '🔇'}
        </button>
    </div>

    <div id="game3d-container" class="game3d-container"></div>

    <div class="controls">
        <div class="control-item">
            <span class="key">SPACE</span> Jump over obstacles
        </div>
        <div class="control-item">
            <span class="key">A/D</span> Change lanes
        </div>
        <div class="control-item">
            <span class="key">S</span> Slide under obstacles
        </div>
        <div class="control-item">
            <span class="key">R</span> Reset game
        </div>
    </div>
</div>

<style>
    .endless-runner-container {
        width: 100%;
        height: 100vh;
        display: flex;
        flex-direction: column;
        background: linear-gradient(135deg, #0a0e1a 0%, #1a1f2e 100%);
        color: white;
        position: relative;
        overflow: hidden;
    }

    .start-screen {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.8);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 1000;
        backdrop-filter: blur(5px);
    }

    .start-content {
        text-align: center;
        padding: 2rem;
        border-radius: 15px;
        background: rgba(255, 255, 255, 0.1);
        border: 2px solid rgba(74, 158, 255, 0.3);
    }

    .start-content h1 {
        font-size: 3rem;
        margin-bottom: 1rem;
        color: #4a9eff;
        text-shadow: 0 0 20px rgba(74, 158, 255, 0.5);
    }

    .start-content p {
        font-size: 1.2rem;
        margin-bottom: 2rem;
        color: #cccccc;
    }

    .game-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem 2rem;
        background: rgba(0, 0, 0, 0.6);
        backdrop-filter: blur(10px);
        z-index: 100;
        border-bottom: 1px solid rgba(74, 158, 255, 0.3);
    }

    .score, .speed, .distance {
        font-size: 1.2rem;
        font-weight: bold;
        color: #4ade80;
        text-shadow: 0 0 10px rgba(74, 222, 128, 0.5);
    }

    .audio-btn {
        background: rgba(0, 0, 0, 0.6);
        border: 2px solid #4a9eff;
        color: white;
        border-radius: 50%;
        width: 40px;
        height: 40px;
        font-size: 1.2rem;
        cursor: pointer;
        transition: all 0.3s;
        backdrop-filter: blur(5px);
    }

    .audio-btn:hover {
        background: rgba(74, 158, 255, 0.3);
        transform: scale(1.1);
    }

    .fps {
        font-size: 1.2rem;
        font-weight: bold;
    }

    .fps-high {
        color: #4ade80;
        text-shadow: 0 0 10px rgba(74, 222, 128, 0.5);
    }

    .fps-medium {
        color: #fbbf24;
        text-shadow: 0 0 10px rgba(251, 191, 36, 0.5);
    }

    .fps-low {
        color: #ef4444;
        text-shadow: 0 0 10px rgba(239, 68, 68, 0.5);
    }

    .game3d-container {
        flex: 1;
        display: flex;
        justify-content: center;
        align-items: center;
        background: #000;
        position: relative;
    }

    .controls {
        position: absolute;
        bottom: 20px;
        left: 20px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        z-index: 100;
    }

    .control-item {
        background: rgba(0, 0, 0, 0.8);
        padding: 8px 12px;
        border-radius: 8px;
        font-size: 14px;
        border: 1px solid rgba(74, 158, 255, 0.3);
        backdrop-filter: blur(5px);
    }

    .key {
        background: linear-gradient(135deg, #4a9eff, #3a8eef);
        color: white;
        padding: 4px 8px;
        border-radius: 4px;
        margin-right: 8px;
        font-family: 'Courier New', monospace;
        font-weight: bold;
        text-shadow: 0 0 5px rgba(74, 158, 255, 0.5);
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.3);
    }

    .controls-preview {
        display: flex;
        gap: 1rem;
        justify-content: center;
        margin-top: 1rem;
    }

    .controls-preview .control-item {
        background: rgba(74, 158, 255, 0.2);
    }

    /* Hide scrollbar for cleaner look */
    ::-webkit-scrollbar {
        display: none;
    }

    * {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
