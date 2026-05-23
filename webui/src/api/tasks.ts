import type {
    CancelTaskResponse,
    CreateTaskRequest,
    CreateTaskResponse,
    GetTaskResponse,
    ListTasksResponse,
} from '../types/api';

const API_BASE = '/api/v1';

export async function createTask(input: CreateTaskRequest): Promise<CreateTaskResponse> {
    const response = await fetch(`${API_BASE}/tasks`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(input),
    });

    if (!response.ok) {
        let detail: string;
        try {
            detail = await response.text();
        } catch {
            detail = '';
        }
        throw new Error(
            `createTask failed => ${response.status} ${response.statusText}:${detail ? ` - ${detail}` : ''}`,
        );
    }

    return (await response.json()) as CreateTaskResponse;
}

export async function getTask(taskId: string): Promise<GetTaskResponse> {
    const response = await fetch(`${API_BASE}/tasks/${taskId}`, {
        method: 'GET',
    });

    if (!response.ok) {
        let detail: string;
        try {
            detail = await response.text();
        } catch {
            detail = '';
        }
        throw new Error(
            `getTask failed => ${response.status} ${response.statusText}:${detail ? ` - ${detail}` : ''}`,
        );
    }

    return (await response.json()) as GetTaskResponse;
}

export async function listTasks(): Promise<ListTasksResponse> {
    const response = await fetch(`${API_BASE}/tasks`, {
        method: 'GET',
    });

    if (!response.ok) {
        let detail: string;
        try {
            detail = await response.text();
        } catch {
            detail = '';
        }
        throw new Error(
            `listTasks failed => ${response.status} ${response.statusText}:${detail ? ` - ${detail}` : ''}`,
        );
    }

    return (await response.json()) as ListTasksResponse;
}

export async function cancelTask(taskId: string): Promise<CancelTaskResponse> {
    const response = await fetch(`${API_BASE}/tasks/${taskId}/cancel`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
    });

    if (!response.ok) {
        let detail: string;
        try {
            detail = await response.text();
        } catch {
            detail = '';
        }
        throw new Error(
            `cancelTask failed => ${response.status} ${response.statusText}:${detail ? ` - ${detail}` : ''}`,
        );
    }

    return (await response.json()) as CancelTaskResponse;
}
