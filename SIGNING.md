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

## Verify and record the public certificate fingerprint

```sh
keytool -list -v -keystore pojarush.jks -alias pojarush
```

Record the SHA-256 certificate fingerprint in a separate secure project record. The fingerprint is public identification data; the keystore and passwords are private signing credentials.

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
