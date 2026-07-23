// Phase 6a shm unit tests (C-level). Only i32 is placeable, so the attacher's
// header-verification and crashed-mid-init timeout paths are UNREACHABLE from
// any HiLow program. They are proven here directly against the runtime, via a
// forged raw segment (hl_shm_test_forge, compiled only under HL_SHM_TEST_SUPPORT)
// and fork(): a child calls hl_scalar_new_i32_placed and the parent checks its
// exit status (a startup error is exit(1)). Scaffolding-grade but permanent
// (the segment header contract, brief §4). Compiled + run (and run under
// valgrind) by tests/shm_unit.rs, which builds runtime.c with
// -DHL_SHM_TEST_SUPPORT and a short init-wait so the timeout case is fast.

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/mman.h>
#include "runtime.h"

// Test-support hooks from runtime.c (only exist under HL_SHM_TEST_SUPPORT).
extern int hl_shm_test_forge(const char* user_name, uint32_t magic, uint32_t layout,
                             uint32_t abi, uint32_t type_tag, uint32_t payload_size,
                             int mark_complete);
extern uint32_t hl_shm_test_magic(void);
extern uint32_t hl_shm_test_layout(void);
extern uint32_t hl_shm_test_abi(void);
extern uint32_t hl_shm_test_type_i32(void);

// Fork a child that attaches to `name` via the real placed constructor. On
// success the child exits with the read i32 value (0..255); a startup error in
// the child is exit(1) from hl_shm_startup_error. Returns the child exit code.
static int child_attach_exit_code(const char* name) {
    fflush(stdout);
    pid_t pid = fork();
    if (pid == 0) {
        // Child: attach. If this returns, verification passed — exit with the
        // observed value so the parent can assert it. If it fails, the runtime
        // has already exit(1)'d.
        HiLowScalar* s = hl_scalar_new_i32_placed(name, 0);
        int32_t v = hl_scalar_get_i32(s);
        hl_scalar_release(s);
        _exit((int)(v & 0xff));
    }
    int status = 0;
    waitpid(pid, &status, 0);
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

static void unlink_seg(const char* user_name) {
    char buf[128];
    snprintf(buf, sizeof(buf), "/hilow.%s", user_name);
    shm_unlink(buf);
}

int main(void) {
    // Case 1: happy attach — a creator writes 42, an attacher observes 42.
    {
        const char* n = "shmunit_happy";
        unlink_seg(n);
        HiLowScalar* creator = hl_scalar_new_i32_placed(n, 42);   // creator
        int code = child_attach_exit_code(n);                     // attacher child
        assert(code == 42 && "attacher should observe the creator's value 42");
        hl_scalar_release(creator);
        unlink_seg(n);
        printf("case_happy_attach: ok\n");
    }

    // Case 2: type-tag mismatch — forged segment holds a non-i32 type.
    {
        const char* n = "shmunit_typemismatch";
        unlink_seg(n);
        int rc = hl_shm_test_forge(n, hl_shm_test_magic(), hl_shm_test_layout(),
                                   hl_shm_test_abi(), 99 /*not i32*/, 4, 1 /*complete*/);
        assert(rc == 0 && "forge should succeed");
        int code = child_attach_exit_code(n);
        assert(code == 1 && "type-tag mismatch must be a startup error (exit 1)");
        unlink_seg(n);
        printf("case_type_mismatch: ok\n");
    }

    // Case 3: bad magic — forged segment is not a HiLow segment.
    {
        const char* n = "shmunit_badmagic";
        unlink_seg(n);
        int rc = hl_shm_test_forge(n, 0xDEADBEEFu, hl_shm_test_layout(),
                                   hl_shm_test_abi(), hl_shm_test_type_i32(), 4, 1);
        assert(rc == 0 && "forge should succeed");
        int code = child_attach_exit_code(n);
        assert(code == 1 && "bad magic must be a startup error (exit 1)");
        unlink_seg(n);
        printf("case_bad_magic: ok\n");
    }

    // Case 4: crashed-mid-init timeout — forged segment never publishes
    // init-complete. The attacher waits the (short, test-configured) bound and
    // fails. Requires a fast HL_SHM_INIT_WAIT_ITERS (set by the driver's -D).
    {
        const char* n = "shmunit_timeout";
        unlink_seg(n);
        int rc = hl_shm_test_forge(n, hl_shm_test_magic(), hl_shm_test_layout(),
                                   hl_shm_test_abi(), hl_shm_test_type_i32(), 4, 0 /*INCOMPLETE*/);
        assert(rc == 0 && "forge should succeed");
        int code = child_attach_exit_code(n);
        assert(code == 1 && "init timeout must be a startup error (exit 1)");
        unlink_seg(n);
        printf("case_init_timeout: ok\n");
    }

    // Case 5: empty segment name — unreachable from HiLow source (empty string
    // literals do not lex), so the runtime's name check is pinned here. attach
    // with "" → hl_shm_object_name returns NULL → startup error (exit 1).
    {
        int code = child_attach_exit_code("");
        assert(code == 1 && "empty segment name must be a startup error (exit 1)");
        printf("case_empty_name: ok\n");
    }

    printf("ALL SHM UNIT TESTS PASSED\n");
    return 0;
}
