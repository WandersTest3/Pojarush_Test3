# Release signing

The release APK must always be signed with the same private keystore. Losing it means Android cannot install future updates over builds signed with that key.

## Create the keystore (Termux)

Run from the repository root:

```sh
keytool -genkeypair -v -keystore pojarush.jks -alias pojarush -keyalg RSA -keysize 2048 -validity 36500
```

Choose and securely store the keystore password and key password. Do not share them or commit them.

The Gradle configuration reads these optional local files from the repository root:

- `.store_password.txt`
- `.key_password.txt`
- `.key_alias.txt`

Create them locally (they are ignored by Git):

```sh
printf '%s' 'YOUR_STORE_PASSWORD' > .store_password.txt
printf '%s' 'YOUR_KEY_PASSWORD' > .key_password.txt
printf '%s' 'pojarush' > .key_alias.txt
```

Alternatively, provide `STORE_PASSWORD`, `KEY_PASSWORD`, and `KEY_ALIAS` as environment variables.

## Current release certificate

The following public certificate identifies the current Pojarush release signing key. Do not publish the keystore or its passwords.

| Field | Value |
|---|---|
| Alias | `pojarush` |
| Owner / Issuer | `CN=Wanders_Test3, O=Pojarush_Test3` |
| Serial number | `ff532dd05ace2577` |
| Created | 2026-11-03 |
| Valid until | 2126-10-10 |
| Public key | RSA 2048-bit |
| Signature algorithm | SHA384withRSA |
| SHA-1 | `65:CF:0D:3D:9B:00:63:7F:6B:15:BC:A6:4E:78:8D:32:AA:EF:37:52` |
| SHA-256 | `82:82:16:21:04:CA:BC:F0:E7:2E:B6:AC:A9:31:33:AD:23:AF:DC:20:BC:FF:41:8E:52:46:D0:DE:CB:E4:33:A4` |

Verify the certificate locally with:

```sh
keytool -list -v -keystore pojarush.jks -alias pojarush
```

## Build a signed release

```sh
./gradlew :Pojarush:assembleRelease
```

The signed APK is written to `Pojarush/build/outputs/apk/release/`.

## Backups and security

- Keep at least two encrypted backups of `pojarush.jks` in separate secure locations.
- Keep the keystore passwords in a password manager.
- Never upload `pojarush.jks`, password files, or signing credentials to GitHub.
- The repository `.gitignore` already excludes `*.jks`, `*.keystore`, and the local password files.
- Never regenerate or replace this keystore for a published app; keep using the same one for every release.
