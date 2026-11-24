// Xcode uses the native target's executable build to decide whether to run its final
// code-signing step. The real executable is built by Cargo in build.sh and replaces this
// placeholder after the native build phases. Keeping one tiny native source makes Xcode treat
// the target as a normal application, so the Cargo-built PhotoComfy binary is signed together
// with its bundle and provisioning profile before it is installed on a device.
int main(void) {
    return 0;
}
