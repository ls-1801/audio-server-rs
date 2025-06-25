use clap::Parser;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
#[derive(Clone, Debug)]
struct Chunk {
    data: Arc<[u8]>,
}

#[derive(clap::Parser)]
struct Args {
    #[clap(short, long)]
    audio_dir: std::path::PathBuf,
    #[clap(short, long, default_value = "16000")]
    sample_rate: u32,
    #[clap(short, long, default_value = "1")]
    channels: u16,
    #[clap(short, long, default_value = "16")]
    bits_per_sample: u16,
    #[clap(short, long, default_value = "1234")]
    port: u16,
    #[clap(short, long, default_value = "128")]
    chunk_size: u32,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // one possible implementation of walking a directory only visiting files
    fn visit_dirs(dir: &std::path::Path, cb: &dyn Fn(&std::fs::DirEntry)) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("wav")) {
                    cb(&entry);
                }
            }
        }
        Ok(())
    }
    let silence_chunk = Chunk {
        data: vec![0; 10 * args.chunk_size as usize].into(),
    };

    let chunks = RefCell::new(vec![]);
    visit_dirs(&args.audio_dir, &|entry| {
        let mut wav = hound::WavReader::open(entry.path()).unwrap();
        let spec = &wav.spec();
        assert_eq!(spec.bits_per_sample, args.bits_per_sample);
        assert_eq!(spec.channels, args.channels);
        assert_eq!(spec.sample_rate, args.sample_rate);

        let data = wav
            .samples::<i16>()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        let file_chunks = data
            .chunks(args.chunk_size as usize)
            .map(|c| Chunk {
                data: Arc::from(bytemuck::cast_slice(c)),
            })
            .collect::<Vec<_>>();
        chunks.borrow_mut().extend(file_chunks);
        chunks.borrow_mut().push(silence_chunk.clone());
        println!("Loaded: {:?}", entry.path());
    })
    .expect("TODO: panic message");

    println!("Loaded {} chunks", chunks.borrow().len());

    let (tx, rx) = tokio::sync::broadcast::channel::<Chunk>(10);
    tokio::spawn(async move {
        let chunks = chunks.take();
        loop {
            for c in &chunks {
                tx.send(c.clone()).unwrap();
                tokio::time::sleep(tokio::time::Duration::from_micros(
                    (1000 * (c.data.len() / 2) / 16) as u64,
                ))
                .await;
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    loop {
        let (mut stream, connection) = listener.accept().await.unwrap();
        println!("Accepted connection from {}", connection);
        tokio::spawn({
            let mut rx = rx.resubscribe();
            async move {
                loop {
                    let chunk = rx.recv().await.unwrap();
                    let Ok(_) = stream.write_all(chunk.data.as_ref()).await else {
                        return;
                    };
                }
            }
        });
    }
}
