// `TcpStream` represents an active TCP connection.
// On the client side we use it to connect TO the server,
// whereas on the server side we received it from incoming connections.
use tokio::net::TcpStream;

// `AsyncBufReadExt` is a trait that gives us the `read_line()` method.
// We use it to read complete lines both from the server and from the terminal.
// Without importing this trait `read_line` would not exist on our BufReader.
use tokio::io::AsyncBufReadExt;

// `AsyncWriteExt` is a trait that gives us the `write_all()` method.
// We use it to send the user's typed messages to the server as raw bytes.
// Without importing this trait `write_all` would not exist on our writer.
use tokio::io::AsyncWriteExt;

// `BufReader` wraps a reader and adds an internal buffer to it.
// Without buffering we'd have to read one byte at a time which is very inefficient.
// BufReader accumulates incoming bytes and lets us read higher level constructs
// like entire lines in one operation.
use tokio::io::BufReader;

// `Arc` stands for "Atomically Reference Counted" - it lets multiple tasks
// share ownership of the same data safely by keeping a count of how many
// owners exist, and only cleaning up when the last owner is gone.
use std::sync::Arc;

// `Mutex` stands for "Mutual Exclusion" - it ensures only one task can
// access the shared data at a time, preventing race conditions.
// We use it here to share the writer between two tasks safely.
use tokio::sync::Mutex;

// This attribute macro transforms our main function into an async one
// powered by the Tokio runtime - the engine that drives all our async code.
#[tokio::main]

// `async fn main()` is our client's entry point. Just like the server,
// we use async so we can handle reading and writing concurrently without blocking.
async fn main() {

    // `TcpStream::connect` initiates a TCP connection to the server.
    // This is the client equivalent of `TcpListener::bind` on the server -
    // instead of waiting for connections it actively creates one.
    // `.await` pauses until the connection is established.
    // `.unwrap()` crashes with an error message if the connection fails -
    // for example if the server isn't running yet.
    let socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    println!("Connected to Chatty Rusty server!");

    // Split the TcpStream into independent read and write halves.
    // - `reader`: used to receive incoming messages FROM the server
    // - `writer`: used to send our messages TO the server
    // We split because we need to use both halves in separate tasks,
    // and Rust's ownership rules don't allow two owners of the same value.
    let (reader, writer) = socket.into_split();

    // Wrap the server read half in a BufReader so we can efficiently
    // read complete lines of text sent by the server.
    let mut server_reader = BufReader::new(reader);

    // `tokio::io::stdin()` is the async version of standard terminal input.
    // We wrap it in a BufReader so we can read complete lines the user types,
    // just like we do with the server reader.
    // Using the async version means waiting for user input won't block other tasks.
    let mut stdin = BufReader::new(tokio::io::stdin());

    // A reusable String buffer that will hold each incoming message from the server.
    // We reuse the same buffer on every iteration to avoid allocating a new
    // String each time, which is more memory efficient.
    let mut server_line = String::new();

    // A reusable String buffer that will hold each line the user types.
    // Same reasoning as above - reuse to avoid unnecessary memory allocations.
    let mut input_line = String::new();

    // We wrap the writer in Arc<Mutex<...>> so it can be safely shared between
    // two tasks - the read task and the write task both need access to it.
    // Arc allows shared ownership, Mutex ensures only one task writes at a time.
    let writer = Arc::new(Mutex::new(writer));

    // Clone the Arc to get a second pointer to the same writer.
    // Remember: this is cheap - it just increments the reference count.
    // We pass this clone into the write task, keeping the original in scope.
    let writer_clone = writer.clone();

    // Spawn a dedicated task for reading messages arriving from the server.
    // This task runs concurrently with the write task below -
    // while this one waits for server messages, the other waits for user input.
    // `async move` transfers ownership of `server_reader` and `server_line`
    // into this task so it can use them independently.
    let read_task = tokio::spawn(async move {
        loop {
            // Wait for a complete line to arrive from the server.
            // `.await` pauses here without blocking - other tasks can run freely.
            match server_reader.read_line(&mut server_line).await {

                // `Ok(0)` means zero bytes were read - the server has disconnected.
                // We notify the user and break out of the loop ending this task.
                Ok(0) => {
                    println!("Server disconnected.");
                    break;
                }

                // `Ok(_)` means we received some bytes - we have a complete line.
                // We use `_` here because we don't need to know how many bytes arrived,
                // we just know the read was successful.
                // We print the message and clear the buffer for the next iteration.
                Ok(_) => {
                    print!("{}", server_line);
                    server_line.clear();
                }

                // `Err` means something went wrong with the connection.
                // We log the error and break out of the loop ending this task.
                Err(e) => {
                    println!("Error reading from server: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn a dedicated task for reading user input from the terminal
    // and forwarding it to the server.
    // `async move` transfers ownership of `stdin`, `input_line`, and `writer_clone`
    // into this task.
    let write_task = tokio::spawn(async move {
        loop {
            // Wait for the user to type a complete line and press Enter.
            // `.await` pauses here without blocking the read task above.
            match stdin.read_line(&mut input_line).await {

                // `Ok(0)` means the user closed terminal input with Ctrl+D on
                // Linux/Mac or Ctrl+Z on Windows - signaling they want to quit.
                Ok(0) => {
                    println!("Disconnecting...");
                    break;
                }

                // `Ok(_)` means the user typed a line successfully.
                // We lock the writer, send the line as bytes to the server,
                // then clear the buffer for the next input.
                Ok(_) => {
                    // Lock the Mutex to get exclusive access to the writer.
                    // `if let Err(e)` means: if write_all returns an error capture
                    // it as `e` and handle it - otherwise do nothing on success.
                    if let Err(e) = writer_clone.lock().await.write_all(input_line.as_bytes()).await {
                        println!("Error sending message: {}", e);
                        break;
                    }
                    input_line.clear();
                }

                // `Err` means something went wrong reading from the terminal.
                // We log the error and break out of the loop ending this task.
                Err(e) => {
                    println!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
    });

    // `tokio::select!` runs both tasks concurrently and waits until the
    // FIRST one finishes. The `_` on each arm means we don't care about
    // the return value of either task - we just want to know one completed.
    // When either task ends - server disconnected or user quit -
    // we stop waiting and the program exits cleanly.
    // This is cleaner than waiting for both tasks since if one ends,
    // continuing the other no longer makes sense.
    tokio::select! {
        _ = read_task => {}
        _ = write_task => {}
    }
}