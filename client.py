import asyncio
import socket
import numpy as np
import sounddevice as sd
import logging

logging.basicConfig(level=logging.DEBUG)
buffer = np.empty((5000, 2), dtype="float32")
async def play_buffer(buffer_shape, buffer_dtype, **kwargs):
    global buffer
    loop = asyncio.get_event_loop()
    event = asyncio.Event()
    idx = 0
    logging.info("Playing buffer ...")

    def callback(outdata, frame_count, time_info, status):
        nonlocal idx
        logging.debug(f"Callback called, idx={idx}")
        if status:
            logging.warning(f"Status: {status}")
        remainder = len(buffer) - idx
        if remainder == 0:
            loop.call_soon_threadsafe(event.set)
            raise sd.CallbackStop
        valid_frames = frame_count if remainder >= frame_count else remainder
        outdata[:valid_frames] = buffer[idx:idx + valid_frames]
        outdata[valid_frames:] = 0
        idx += valid_frames
        # Update buffer
        buffer = buffer[valid_frames:]
        logging.debug(f"Played {valid_frames} frames, idx={idx}")

    stream = sd.OutputStream(callback=callback, dtype=buffer_dtype, channels=40000, **kwargs)
    with stream:
        await event.wait()
        event.clear()
        logging.debug("Event set, buffer playback finished")

async def receive_and_play(queue, sock, buffer_nbytes):
    extra_data = b''
    global buffer

    while True:
        data, _ = sock.recvfrom(buffer_nbytes)
        logging.debug(f"Received {len(data)} bytes of data")
        data = extra_data + data
        while len(data) >= buffer_nbytes:
            buffer = np.frombuffer(data[:buffer_nbytes], dtype="float32").reshape(buffer.shape)
            logging.debug(f"Buffer filled, extra_data size: {len(extra_data)}")
        extra_data = data
        logging.debug(f"Accumulating data, extra_data size: {len(extra_data)}")

async def main(frames=5000, channels=2, dtype="float32", ip="127.0.0.1", port=12345, **kwargs):
    buffer_shape = (frames, channels)
    buffer_dtype = dtype
    buffer_nbytes = np.empty(buffer_shape, dtype=buffer_dtype).nbytes
    queue = asyncio.Queue()
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((ip, port))
    logging.info("Receiving and playing buffer ...")

    receive_task = asyncio.create_task(receive_and_play(queue, sock, buffer_nbytes))
    play_task = asyncio.create_task(play_buffer(queue, buffer_shape, buffer_dtype, **kwargs))

    await receive_task
    await queue.put(None)  # Signal the play_buffer to stop
    await play_task
    logging.info("Done")

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logging.info("Interrupted by user")