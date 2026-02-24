// We're importing TcpListener from Tokio's networking module.
// TcpListener is what allows our server to "listen" for incoming TCP connections,
// just like a receptionist waiting for visitors to arrive.
use tokio::net::TcpListener;

// `Arc` stands for "Atomically Reference Counted" - it lets multiple parts of
// your program share ownership of the same data safely.
// Think of it as a shared pointer that keeps a count of how many owners exist,
// and only cleans up the data when the last owner is gone.
use std::sync::Arc;

// `Mutex` stands for "Mutual Exclusion" - it ensures only one task can
// access the shared data at a time, preventing race conditions.
// A race condition is when two tasks try to modify the same data simultaneously,
// causing unpredictable bugs.
use tokio::sync::Mutex;

// `HashMap` is a key-value store - we'll use it to store all connected clients.
// Each client will have a unique key (their address) and a value (their socket).
use std::collections::HashMap;

// `TcpStream` represents an active TCP connection with a client.
// Once a client connects, all communication happens through a TcpStream.
use tokio::net::TcpStream;

// `OwnedWriteHalf` is one half of a split TcpStream - the writing half.
// Tokio allows us to split a TcpStream into a read half and a write half.
// This is very useful because we want to:
// - Read incoming messages from a client on one side
// - Write outgoing messages to a client on the other side
// We store only the write halves in our shared db, because that's what we need
// to forward messages TO clients.
use tokio::net::tcp::OwnedWriteHalf;

// `BufReader` wraps a reader and adds an internal buffer to it.
// Without buffering we'd have to read one byte at a time which is very inefficient.
// BufReader accumulates incoming bytes and lets us read higher level constructs
// like entire lines in one operation.
use tokio::io::BufReader;

// `AsyncBufReadExt` is a trait that extends BufReader with async methods.
// Specifically it gives us the `read_line()` method we use to read a full
// line of text from a client. Without importing this trait, `read_line`
// would simply not exist on our BufReader.
// A trait in Rust is a collection of methods that a type can implement -
// similar to interfaces in other languages.
use tokio::io::AsyncBufReadExt;

// `AsyncWriteExt` is a trait that gives us async write methods on our write half.
// Specifically it provides `write_all()` which we use to send messages to clients.
// Just like AsyncBufReadExt gave us `read_line()` for reading,
// AsyncWriteExt gives us `write_all()` for writing.
use tokio::io::AsyncWriteExt;

// We define a type alias called `Db` to avoid writing this long type everywhere.
// Breaking it down from the inside out:
// - `OwnedWriteHalf`: the writing half of a split TcpStream. We only store the
//   write half because that's all we need to forward messages TO a client.
//   The read half stays inside each client's own task, where it reads incoming messages.
// - `HashMap<String, OwnedWriteHalf>`: maps a client's address (as text) to their write half
// - `Mutex<...>`: wraps the HashMap so only one task can access it at a time
// - `Arc<...>`: allows multiple tasks to share ownership of the Mutex
// Together, Arc<Mutex<...>> is the classic Rust pattern for shared mutable state.
type Db = Arc<Mutex<HashMap<String, OwnedWriteHalf>>>;

// This attribute macro transforms our regular main function into an async one
// powered by the Tokio runtime. Rust by default doesn't know how to run async code -
// Tokio provides the engine that actually drives it.
// Think of it as starting a car engine before you can drive.
#[tokio::main]

// `async fn main()` is our program's entry point. The `async` keyword means this
// function can perform non-blocking operations - it can "pause and wait" for things
// like network connections without freezing the entire program.
async fn main() {

    // `TcpListener::bind(...)` tells the OS: "I want to receive TCP connections
    // on this IP address and port." 
    // - "127.0.0.1" is localhost, meaning only connections from this same machine.
    // - "8080" is the port number we chose (like a specific door in a building).
    // `.await` pauses here until the OS confirms the port is reserved.
    // `.unwrap()` means: "if this fails, crash immediately with an error message."
    // In production code you'd handle errors more gracefully, but this is fine for learning.
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    // Simply print a message to the terminal so we know the server started successfully.
    println!("Chatty Rusty server listening on 127.0.0.1:8080");

    // Create a new empty HashMap, wrap it in a Mutex, then wrap that in an Arc.
    // This is our shared client registry - every connected client will be stored here.
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    // `loop` is Rust's infinite loop - it runs forever until the program is killed.
    // Our server should always be running and ready to accept new connections,
    // so an infinite loop is exactly what we want here.
    loop {

        // `listener.accept()` waits for a new client to connect.
        // `.await` pauses here (without blocking other work) until someone connects.
        // When a connection arrives, it returns two things which we unpack:
        // - `socket`: the communication channel with that specific client
        // - `addr`: the client's IP address and port (e.g. "127.0.0.1:54321")
        // `.unwrap()` again crashes on error - acceptable for now.
        let (socket, addr) = listener.accept().await.unwrap();

        // Print the new client's address so we can see who connected.
        // The `{}` is Rust's placeholder for displaying a value, similar to
        // Python's f-strings or JavaScript's template literals.
        println!("New connection from: {}", addr);

        // Before we can pass `db` into a new task, we need to clone the Arc.
        // IMPORTANT: cloning an Arc does NOT copy the underlying data -
        // it just creates a new pointer to the same data and increments the reference count.
        // This is cheap and is the intended way to share an Arc across tasks.
        let db_clone = db.clone();

        // `tokio::spawn` launches a new task to handle this client independently.
        // The `async move` block creates an async closure that takes ownership
        // of the variables it uses - in this case `socket`, `addr`, and `db_clone`.
        // `move` means the task owns these values, not the main loop.
        // This is necessary because the main loop continues immediately to wait
        // for the next connection, so we can't borrow - we must transfer ownership.
        tokio::spawn(async move {
            handle_client(socket, addr.to_string(), db_clone).await;
        });

    } // Back to the top of the loop - wait for the next connection
}

// This function will handle an individual client connection.
// It receives:
// - `socket`: the full TcpStream for this client
// - `addr`: the client's address as a String, used as their unique identifier
// - `db`: the shared registry of all connected clients
async fn handle_client(socket: TcpStream, addr: String, db: Db) {
    // `into_split()` consumes the TcpStream and splits it into two independent halves:
    // - `reader`: we use this to READ messages coming FROM this client
    // - `writer`: we store this in db so other tasks can WRITE messages TO this client
    let (reader, writer) = socket.into_split();

    // `BufReader` wraps our read half and adds buffering to it.
    // Without buffering, we'd have to read one byte at a time which is very inefficient.
    // BufReader accumulates incoming bytes into an internal buffer and lets us
    // read higher level constructs - like entire lines - in one operation.
    let mut buf_reader = BufReader::new(reader);

    // We create an empty String that will be reused on each iteration to hold
    // the current line being read. Using `mut` because its content will change.
    let mut line = String::new();

    // Lock the Mutex to get exclusive access to the HashMap, then insert this
    // client's write half. `.lock().await` pauses until the lock is available.
    // The lock is automatically released when `db` goes out of scope at the end
    // of this block - this is Rust's ownership system keeping things safe.
    db.lock().await.insert(addr.clone(), writer);

    println!("{} has been added to the client registry", addr);

    // This loop keeps running as long as the client is connected.
    // Each iteration waits for a complete line of text from the client.
    loop {
        // `read_line` reads bytes from the buffer until it hits a newline character `\n`
        // and appends the result into our `line` String.
        // It returns a Result containing how many bytes were read.
        // `.await` pauses here until a full line arrives - during this pause
        // Tokio can run other tasks on this thread freely.
        match buf_reader.read_line(&mut line).await {
            // `Ok(0)` means zero bytes were read - this is how TCP signals
            // that the client has disconnected. We break out of the loop.
            Ok(0) => {
                println!("{} disconnected", addr);
                break;
            }
            // `Ok(n)` means we successfully read n bytes - we have a complete line!
            Ok(n) => {
                println!("Received {} bytes from {}: {}", n, addr, line.trim());

                // Format the message to include the sender's address so other clients
                // know who sent it. `format!` works like `println!` but returns a String
                // instead of printing it - we store it in `msg` to send to everyone.
                let msg = format!("{}: {}", addr, line);

                // Lock the db to get access to all connected clients' write halves.
                // We need to iterate over every client and send them the message.
                let mut db_lock = db.lock().await;

                // `iter_mut()` gives us a mutable iterator over all key-value pairs in the HashMap.
                // We need mutability because writing to a TcpStream modifies its internal state.
                for (client_addr, writer) in db_lock.iter_mut() {

                    // We skip the sender - they don't need to receive their own message back.
                    // `*client_addr` dereferences the reference to compare it with `addr`.
                    if *client_addr != addr {

                        // `write_all` sends the entire message bytes to this client.
                        // `.as_bytes()` converts our String into raw bytes since TCP works
                        // with bytes not text.
                        // `if let Err(e)` means: "if this returns an error, capture it as e"
                        // and handle it - otherwise do nothing on success.
                        if let Err(e) = writer.write_all(msg.as_bytes()).await {
                            println!("Error sending message to {}: {}", client_addr, e);
                        }
                    }
                }

                // Release the lock by dropping it explicitly before we clear the line.
                // Holding a lock longer than necessary blocks other tasks from accessing db.
                // This is good practice - always hold locks for the shortest time possible.
                drop(db_lock);

                // We must clear the line buffer after each read, otherwise the next
                // read_line call will APPEND to the existing content instead of
                // replacing it, giving us garbled messages.
                line.clear();
            }
            // `Err` means something went wrong with the connection - e.g. the client
            // crashed or the network dropped. We log it and break out of the loop.
            Err(e) => {
                println!("Error reading from {}: {}", addr, e);
                break;
            }
        }
    }

    // When the loop ends the client has disconnected. We remove them from the
    // registry so we don't try to forward messages to a dead connection.
    db.lock().await.remove(&addr);
    println!("{} has been removed from the client registry", addr);
}