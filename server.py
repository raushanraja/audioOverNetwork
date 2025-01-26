import asyncio
import socket
import numpy as np
import sounddevice as sd

async def record_and_send(sock, addr, **kwargs):
    loop = asyncio.get_event_loop()

    def callback(indata, frame_count, time_info, status):
        if status:
            print(status)
        sock.sendto(indata.tobytes(), addr)

    stream = sd.InputStream( callback=callback,  **kwargs)
    with stream:
        await asyncio.sleep(3600)  # Keep the stream open for 1 hour

async def main(frames=4000, channels=2, dtype="float32", ip="127.0.0.1", port=12345, **kwargs):
    buffer = np.empty((frames, channels), dtype=dtype)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    addr = (ip, port)
    print("recording and sending buffer ...")
    await record_and_send(sock, addr, dtype=dtype, channels=channels, **kwargs)
    print("done")

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nInterrupted by user")