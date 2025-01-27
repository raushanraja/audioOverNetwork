const byteString =
    '\0\0\0\0\0\0\x80?\0\0\0@\0\0@@\0\0\x80@\0\0\xa0@\0\0\xc0@\0\0\xe0@\0\0\0A\0\0\x10A';

// Convert the string to a Uint8Array by getting the char codes of each character
const byteArray = new Uint8Array(
    byteString.split('').map((c) => c.charCodeAt(0))
);

console.log(byteArray);

// Create a DataView to read from the byteArray
const dataView = new DataView(byteArray.buffer);

// Parse the values from the byte array using DataView
const values = [];

for (let i = 0; i < byteArray.length; i += 4) {
    values.push(dataView.getFloat32(i, true)); // true indicates little-endian byte order
}
console.log(values);

// // Convert byteString to a binary `Blob`
// const blob = new Blob([byteString], { type: 'application/octet-stream' });
//
// // Create a FileReader to read the Blob's content
// const reader = new FileReader();
// reader.onload = function () {
//     const byteArray = new Uint8Array(reader.result);
//     console.log(byteArray); // Logs the raw byte array
// };
//
// reader.readAsArrayBuffer(blob);

function binaryToArrayBuffer(binary) {
    const buffer = new ArrayBuffer(binary.length);
    const view = new Uint8Array(buffer);
    for (let i = 0; i < binary.length; i++) {
        view[i] = binary.charCodeAt(i);
    }
    return buffer;
}

let buffer = binaryToArrayBuffer(byteString);
console.log(buffer);
const dataView2 = new DataView(buffer);
const values2 = [];

for (let i = 0; i < buffer.byteLength; i += 4) {
    values2.push(dataView2.getFloat32(i, true)); // true indicates little-endian byte order
}
console.log(values2);

const ba = new Uint8Array(await (await new Response(byteString)).arrayBuffer());
console.log(ba);
