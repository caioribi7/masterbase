#!/usr/bin/env python3
"""SMTP falso para testes: aceita tudo e grava cada e-mail (já decodificado) num arquivo.
Uso: python3 dev/smtp-sink.py PORTA ARQUIVO"""
import asyncio
import email
import email.policy
import sys

PORT, OUT = int(sys.argv[1]), sys.argv[2]


async def session(reader, writer):
    def say(line):
        writer.write((line + "\r\n").encode())

    say("220 sink")
    while line := await reader.readline():
        cmd = line.decode(errors="replace").strip().upper()
        if cmd.startswith(("EHLO", "HELO")):
            say("250 sink")
        elif cmd == "DATA":
            say("354 manda")
            data = b""
            while (chunk := await reader.readline()) not in (b".\r\n", b""):
                data += chunk
            msg = email.message_from_bytes(data, policy=email.policy.default)
            with open(OUT, "a", encoding="utf-8") as f:
                f.write(f"To: {msg['to']}\nSubject: {msg['subject']}\n{msg.get_content()}\n----\n")
            say("250 ok")
        elif cmd == "QUIT":
            say("221 tchau")
            await writer.drain()
            break
        else:
            say("250 ok")
        await writer.drain()
    writer.close()


async def main():
    server = await asyncio.start_server(session, "127.0.0.1", PORT)
    async with server:
        await server.serve_forever()


asyncio.run(main())
