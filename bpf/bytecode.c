#define KBUILD_MODNAME "bytecode"
#include <linux/kconfig.h>
#include <linux/bpf.h>
#include <linux/tcp.h>
#include <linux/version.h>
#include "bpf_helpers.h"

#ifndef LINUX_VERSION_CODE
#pragma message("LINUX_VERSION_CODE not defined")
#endif

#define bpf_printk(fmt, ...)                             \
({                                                       \
    char _fmt[] = fmt;                                   \
    bpf_trace_printk(_fmt, sizeof(_fmt), ##__VA_ARGS__); \
})

enum kind {
    EXEC,
    EXIT,
    CONNECT,
    ACCEPT,
    CLOSE,
};

struct header {
    u32 kind;
    u32 pid;
};

struct sock4 {
    u32 proto;
    u32 saddr;
    u32 sport;
    u32 daddr;
    u32 dport;
};

struct connect {
    struct header header;
    struct sock4  socket;
};

struct accept {
    struct header header;
    struct sock4  socket;
};

struct close {
    struct header header;
    struct sock4  socket;
};

SEC("maps/events")
struct bpf_map_def events = {
    .type        = BPF_MAP_TYPE_PERF_EVENT_ARRAY,
    .key_size    = sizeof(int),
    .value_size  = sizeof(int),
    .max_entries = 512,
};

SEC("maps/socks")
struct bpf_map_def socks = {
    .type        = BPF_MAP_TYPE_HASH,
    .key_size    = sizeof(u32),
    .value_size  = sizeof(struct sock *),
    .max_entries = 512,
};

SEC("kprobe/call-tcp-connect")
int bpf_call_tcp_connect(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_PARM1(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    bpf_map_update_elem(&socks, &tid, &sk, 0);

    return 0;
}

SEC("kretprobe/exit-tcp-connect")
int bpf_exit_tcp_connect(struct pt_regs *ctx) {
    int rc = PT_REGS_RC(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock **skp = bpf_map_lookup_elem(&socks, &tid);
    if (skp == 0) {
        return 0;
    }

    if (rc != 0) {
        bpf_map_delete_elem(&socks, &tid);
        return 0;
    }

    struct sock *sk = *skp;
    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct connect event = {
        .header = {
            .kind = CONNECT,
            .pid  = pid,
        },
        .socket = {
            .proto = 6,
            .saddr = sc.skc_rcv_saddr,
            .sport = sc.skc_num,
            .daddr = sc.skc_daddr,
            .dport = ntohs(sc.skc_dport),
        },
    };

    rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("connect event output failure: %d\n", rc);
    }
    bpf_map_delete_elem(&socks, &tid);


    return 0;
}

SEC("kretprobe/inet_csk_accept")
int bpf_call_inet_csk_accept(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_RC(ctx);

    if (sk == NULL) {
        return 0;
    }

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct accept event = {
        .header = {
            .kind = ACCEPT,
            .pid  = pid,
        },
        .socket = {
            .proto = 6,
            .saddr = sc.skc_rcv_saddr,
            .sport = sc.skc_num,
            .daddr = sc.skc_daddr,
            .dport = ntohs(sc.skc_dport),
        },
    };

    int rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("accept event output failure: %d\n", rc);
    }

    return 0;
}

SEC("kprobe/tcp_close")
int bpf_call_tcp_close(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_PARM1(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct close event = {
        .header = {
            .kind = CLOSE,
            .pid  = pid,
        },
        .socket = {
            .proto = 6,
            .saddr = sc.skc_rcv_saddr,
            .sport = sc.skc_num,
            .daddr = sc.skc_daddr,
            .dport = ntohs(sc.skc_dport),
        },
    };

    int rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("close event output failure: %d\n", rc);
    }
    bpf_map_delete_elem(&socks, &tid);

    return 0;
}

typedef struct {
    u64   __pad;
    char  *filename;
    pid_t pid;
    pid_t old_pid;
} sched_process_exec_ctx;

SEC("tracepoint/sched/sched_process_exec")
int bpf_trace_sched_process_exec(sched_process_exec_ctx *ctx) {
    struct header event = {
        .kind = EXEC,
        .pid  = ctx->pid,
    };

    int rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("exec event output failure: %d\n", rc);
    }

    return 0;
}

typedef struct {
    u64   __pad;
    char  comm[16];
    pid_t pid;
    int   prio;
} sched_process_exit_ctx;

SEC("tracepoint/sched/sched_process_exit")
int bpf_trace_sched_process_exit(sched_process_exit_ctx *ctx) {
    struct header event = {
        .kind = EXIT,
        .pid  = ctx->pid,
    };

    int rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("exit event output failure: %d\n", rc);
    }

    return 0;
}

char  _license[] SEC("license") = "GPL";
__u32 _version   SEC("version") = LINUX_VERSION_CODE;
