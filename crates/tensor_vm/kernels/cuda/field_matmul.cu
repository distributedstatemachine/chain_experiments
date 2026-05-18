#include <cuda_runtime.h>
#include <stdint.h>

namespace {

constexpr uint64_t kModulus = 2147483647ULL;

__device__ uint64_t field_mul(uint64_t lhs, uint64_t rhs) {
    return ((lhs % kModulus) * (rhs % kModulus)) % kModulus;
}

__global__ void field_matmul_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t rows,
    uint64_t inner,
    uint64_t cols) {
    uint64_t cell = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t total = rows * cols;
    if (cell >= total) {
        return;
    }

    uint64_t row = cell / cols;
    uint64_t col = cell % cols;
    uint64_t acc = 0;
    for (uint64_t k = 0; k < inner; ++k) {
        acc += field_mul(lhs[row * inner + k], rhs[k * cols + col]);
        acc %= kModulus;
    }
    out[cell] = acc;
}

int fail(cudaError_t status, int code) {
    return status == cudaSuccess ? 0 : code;
}

}  // namespace

extern "C" int tensor_vm_cuda_device_count(uint32_t* out) {
    if (out == nullptr) {
        return -1;
    }
    int count = 0;
    cudaError_t status = cudaGetDeviceCount(&count);
    if (status != cudaSuccess) {
        *out = 0;
        return -2;
    }
    *out = static_cast<uint32_t>(count);
    return 0;
}

extern "C" int tensor_vm_cuda_field_matmul(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t rows,
    uint64_t inner,
    uint64_t cols) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_count = 0;
    cudaError_t status = cudaGetDeviceCount(&device_count);
    if (status != cudaSuccess || device_index >= static_cast<uint32_t>(device_count)) {
        return -6;
    }
    status = cudaSetDevice(static_cast<int>(device_index));
    if (status != cudaSuccess) {
        return -6;
    }
    if (rows != 0 && cols > UINT64_MAX / rows) {
        return -2;
    }
    if (inner != 0 && rows > UINT64_MAX / inner) {
        return -2;
    }
    if (cols != 0 && inner > UINT64_MAX / cols) {
        return -2;
    }

    uint64_t lhs_len = rows * inner;
    uint64_t rhs_len = inner * cols;
    uint64_t out_len = rows * cols;
    if (out_len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    size_t lhs_bytes = static_cast<size_t>(lhs_len * sizeof(uint64_t));
    size_t rhs_bytes = static_cast<size_t>(rhs_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));

    status = cudaMalloc(&device_lhs, lhs_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, rhs_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, lhs_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, rhs_bytes, cudaMemcpyHostToDevice);
    }
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        cudaFree(device_out);
        return -4;
    }

    constexpr uint64_t threads_per_block = 256;
    uint64_t blocks = (out_len + threads_per_block - 1) / threads_per_block;
    field_matmul_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
        device_lhs,
        device_rhs,
        device_out,
        rows,
        inner,
        cols);
    status = cudaGetLastError();
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);

    return fail(status, -5);
}
